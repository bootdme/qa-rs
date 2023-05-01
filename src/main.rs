use dotenv::dotenv;
use hyper::{
    Body, Request, Response, Server, StatusCode,
    service::{make_service_fn, service_fn},
    server::conn::AddrStream,
};

use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{net::SocketAddr, sync::Arc};

use std::collections::HashMap;

async fn handle_request(pool: Arc<PgPool>, req: Request<Body>) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    match (req.method(), req.uri().path()) {
        (&hyper::Method::GET, "/api/v1/questions") => {
            let params: HashMap<String, String> = parse_query_parameters(req.uri().query());

            if let Some(product_id) = params.get("product_id").and_then(|v| v.parse::<i32>().ok()) {
                let (page, count) = get_page_count(&params);

                for key in params.keys() {
                    if key != "product_id" && key != "page" && key != "count" {
                        return Ok(Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(format!("Unexpected query parameter: {}", key).into())
                            .unwrap());
                    }
                }

                get_questions(pool, product_id, page, count).await
            } else {
                Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body("Missing product_id query parameter".into())
                    .unwrap())
            }
        }
        (&hyper::Method::GET, path) if path.starts_with("/api/v1/answers/") => {
            let question_id = path.strip_prefix("/api/v1/answers/").and_then(|v| v.parse::<i32>().ok());
            if let Some(question_id) = question_id {
                let params: HashMap<String, String> = parse_query_parameters(req.uri().query());
                let (page, count) = get_page_count(&params);

                for key in params.keys() {
                    if key != "page" && key != "count" {
                        return Ok(Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(format!("Unexpected query parameter: {}", key).into())
                            .unwrap());
                    }
                }
                get_answers(pool, question_id, page, count).await
            } else {
                Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body("Invalid question_id path parameter".into())
                    .unwrap())
            }
        }
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("Not found".into())
            .unwrap()),
    }
}

fn parse_query_parameters(query: Option<&str>) -> HashMap<String, String> {
    query.map(|q| {
        q.split('&')
            .filter_map(|p| {
                let mut parts = p.split('=');
                let key = parts.next().unwrap_or("").to_string();
                let value = parts.next().unwrap_or("").to_string();
                if key.is_empty() || value.is_empty() {
                    None
                } else {
                    Some((key, value))
                }
            })
            .collect()
    }).unwrap_or_default()
}

fn get_page_count(params: &HashMap<String, String>) -> (i32, i32) {
    let page = params.get("page").and_then(|v| v.parse::<i32>().ok()).unwrap_or(1);
    let count = params.get("count").and_then(|v| v.parse::<i32>().ok()).unwrap_or(5);
    (page, count)
}

async fn get_questions(pool: Arc<PgPool>, product_id: i32, page: i32, count: i32) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    let limit = (page * count) as i64;

    let row = sqlx::query!(
        r#"
        SELECT
            Json_agg(
                Json_build_object(
                    'question_id',          q.id,
                    'question_body',        q.body,
                    'question_date',        q.date_written,
                    'asker_name',           q.asker_name,
                    'question_helpfulness', q.helpful,
                    'reported',             q.reported,
                    'answers', (
                        SELECT COALESCE(a, '{}'::json)
                        FROM (
                            SELECT Json_object_agg(
                                a.id,
                                Json_build_object(
                                    'id',            a.id,
                                    'body',          a.body,
                                    'date',          a.date_written,
                                    'answerer_name', a.answerer_name,
                                    'helpfulness',   a.helpful,
                                    'photos', (
                                        SELECT COALESCE(p, '[]'::json)
                                        FROM (
                                            SELECT
                                                Json_agg(
                                                    Json_build_object(
                                                        'id',  ap.id,
                                                        'url', ap.url
                                                    )
                                                ) AS p
                                            FROM answer_photos AS ap
                                            WHERE ap.answer_id = a.id
                                        ) AS myPhotos
                                    )
                                )
                            ) AS a
                            FROM answers a
                            WHERE a.question_id = q.id
                        ) AS myAnswers
                    )
                )
            ) AS results
        FROM (
            SELECT *
            FROM questions
            WHERE product_id = $1
            LIMIT $2
        ) AS q;
        "#,
        product_id,
        limit
    )
    .fetch_optional(&*pool)
    .await
    .map_err(|e| {
        println!("Failed to fetch data from the database: {:?}", e);
        e
    })?;

    let response = if let Some(row) = row {
        serde_json::from_value(row.results.into()).unwrap_or_else(|_| serde_json::Value::Array(vec![]))
    } else {
        serde_json::Value::Array(vec![])
    };

    // Check for null value and return an empty array if response is null
    let response = if response == serde_json::Value::Null {
        serde_json::Value::Array(vec![])
    } else {
        response
    };

    Ok(Response::builder()
        .header("content-type", "application/json")
        .body(Body::from(response.to_string()))
        .unwrap())
}

async fn get_answers(pool: Arc<PgPool>, question_id: i32, page: i32, count: i32) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    let limit = (page * count) as i64;

    let row = sqlx::query!(
        r#"
        SELECT
            Json_agg(
                Json_build_object(
                    'answer_id',     a.id,
                    'body',          a.body,
                    'date',          a.date_written,
                    'answerer_name', a.answerer_name,
                    'helpfulness',   a.helpful,
                    'photos', (
                        SELECT COALESCE(Json_agg(d), '[]'::json)
                        FROM (
                            SELECT
                                ap.id,
                                ap.url
                            FROM answer_photos ap
                            WHERE ap.answer_id = a.id
                        ) d
                    ) 
                )
            ) AS results
        FROM answers a
        WHERE a.question_id = $1
        LIMIT $2
        "#,
        question_id,
        limit
    )
    .fetch_optional(&*pool)
    .await
    .map_err(|e| {
        println!("Failed to fetch data from the database: {:?}", e);
        e
    })?;

    let response = if let Some(row) = row {
        serde_json::from_value(row.results.into()).unwrap_or_else(|_| serde_json::Value::Array(vec![]))
    } else {
        serde_json::Value::Array(vec![])
    };

    // Check for null value and return an empty array if response is null
    let response = if response == serde_json::Value::Null {
        serde_json::Value::Array(vec![])
    } else {
        response
    };

    Ok(Response::builder()
        .header("content-type", "application/json")
        .body(Body::from(response.to_string()))
        .unwrap())

}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")?;

    let pool = PgPoolOptions::new().max_connections(5).connect(&database_url).await?;
    let pool = Arc::new(pool);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let make_svc = make_service_fn(move |_conn: &AddrStream| {
        let pool = pool.clone();
        async { Ok::<_, hyper::Error>(service_fn(move |req| handle_request(pool.clone(), req))) }
    });

    let server = Server::bind(&addr).serve(make_svc);

    println!("Listening on http://{}", addr);

    server.await?;

    Ok(())
}
