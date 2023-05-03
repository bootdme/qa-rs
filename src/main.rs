use dotenv::dotenv;
use hyper::{
    Body, Request, Response, Server, StatusCode,
    service::{make_service_fn, service_fn},
    server::conn::AddrStream,
};

use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{net::SocketAddr, sync::Arc};
use serde::Deserialize;

use std::collections::HashMap;

#[derive(Deserialize)]
struct NewQuestion {
    body: String,
    name: String,
    email: String,
    product_id: i32,
}

#[derive(Deserialize)]
struct NewAnswer {
    body: String,
    name: String,
    email: String,
    photos: Vec<String>,
}

async fn handle_request(pool: Arc<PgPool>, req: Request<Body>) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    match (req.method(), req.uri().path()) {
        (&hyper::Method::GET, "/api/v1/questions") => {
            let params: HashMap<String, String> = parse_query_parameters(req.uri().query());

            if let Some(product_id) = params.get("product_id").and_then(|v| v.parse::<i32>().ok()) {
                let (page, count) = get_page_count(&params);

                for key in params.keys() {
                    if key != "product_id" && key != "page" && key != "count" {
                        return create_error_response(StatusCode::BAD_REQUEST, format!("Unexpected query parameter: {}", key));
                    }
                }

                get_questions(pool, product_id, page, count).await
            } else {
                if !params.contains_key("product_id") {
                    return create_error_response(StatusCode::BAD_REQUEST, "Missing product_id query parameter".to_string())
                }
                return create_error_response(StatusCode::BAD_REQUEST, "Invalid product_id query parameter".to_string());
            }
        }
        (&hyper::Method::GET, path) if path.starts_with("/api/v1/questions/") && path.ends_with("/answers") => {
            let question_id_str = path.strip_prefix("/api/v1/questions/").unwrap();
            let question_id_str = question_id_str.strip_suffix("/answers").unwrap();

            if let Ok(question_id) = question_id_str.parse::<i32>() {
                let params: HashMap<String, String> = parse_query_parameters(req.uri().query());
                let (page, count) = get_page_count(&params);

                for key in params.keys() {
                    if key != "page" && key != "count" {
                        return create_error_response(StatusCode::BAD_REQUEST, format!("Unexpected query parameter: {}", key));
                    }
                }

                get_answers(pool, question_id, page, count).await
            } else {
                return create_error_response(StatusCode::BAD_REQUEST, "Invalid question_id path parameter".into());
            }
        }
        (&hyper::Method::POST, "/api/v1/questions") => {
            let body_bytes = hyper::body::to_bytes(req.into_body()).await?;
            let body_str = String::from_utf8(body_bytes.to_vec())?;

            let question_data: Result<NewQuestion, _> = serde_json::from_str(&body_str);
            if let Ok(question_data) = question_data {
                add_question(pool, question_data).await
            } else {
                return create_error_response(StatusCode::BAD_REQUEST, "Invalid request body".into());
            }
        }
        (&hyper::Method::POST, path) if path.starts_with("/api/v1/questions/") && path.ends_with("/answers") => {
            let question_id = path
                .strip_prefix("/api/v1/questions/")
                .and_then(|v| v.strip_suffix("/answers"))
                .and_then(|v| v.parse::<i32>().ok());

            if let Some(question_id) = question_id {
                let body_bytes = hyper::body::to_bytes(req.into_body()).await?;
                let body_str = String::from_utf8(body_bytes.to_vec())?;

                let answer_data: Result<NewAnswer, _> = serde_json::from_str(&body_str);
                if let Ok(answer_data) = answer_data {
                    add_answer(pool, question_id, answer_data).await
                } else {
                    return create_error_response(StatusCode::BAD_REQUEST, "Invalid request body".into());
                }
            } else {
                return create_error_response(StatusCode::BAD_REQUEST, "Invalid question_id path parameter".into());
            }
        }
        (&hyper::Method::PUT, path) if path.starts_with("/api/v1/questions/") && path.ends_with("/helpful") => {
            let question_id = path
                .strip_prefix("/api/v1/questions/")
                .and_then(|v| v.strip_suffix("/helpful"))
                .and_then(|v| v.parse::<i32>().ok());

            if let Some(question_id) = question_id {
                update_question_helpful(pool, question_id).await
            } else {
                return create_error_response(StatusCode::BAD_REQUEST, "Invalid question_id path parameter".into());
            }
        }
        (&hyper::Method::PUT, path) if path.starts_with("/api/v1/questions/") && path.ends_with("/report") => {
            let question_id = path
                .strip_prefix("/api/v1/questions/")
                .and_then(|v| v.strip_suffix("/report"))
                .and_then(|v| v.parse::<i32>().ok());

            if let Some(question_id) = question_id {
                update_question_report(pool, question_id).await
            } else {
                return create_error_response(StatusCode::BAD_REQUEST, "Invalid question_id path parameter".into());
            }
        }
        (&hyper::Method::PUT, path) if path.starts_with("/api/v1/answers/") && path.ends_with("/helpful") => {
            let answer_id = path
                .strip_prefix("/api/v1/answers/")
                .and_then(|v| v.strip_suffix("/helpful"))
                .and_then(|v| v.parse::<i32>().ok());

            if let Some(answer_id) = answer_id {
                update_answer_helpful(pool, answer_id).await
            } else {
                return create_error_response(StatusCode::BAD_REQUEST, "Invalid answer_id path parameter".into());
            }
        }
        (&hyper::Method::PUT, path) if path.starts_with("/api/v1/answers/") && path.ends_with("/report") => {
            let answer_id = path
                .strip_prefix("/api/v1/answers/")
                .and_then(|v| v.strip_suffix("/report"))
                .and_then(|v| v.parse::<i32>().ok());

            if let Some(answer_id) = answer_id {
                update_answer_report(pool, answer_id).await
            } else {
                return create_error_response(StatusCode::BAD_REQUEST, "Invalid answer_id path parameter".into());
            }
        }
        _ => return create_error_response(StatusCode::NOT_FOUND, "Path not found".into()),
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

fn create_success_response(status: StatusCode, body: serde_json::Value) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::builder()
       .status(status)
       .header("content-type", "application/json")
       .body(Body::from(body.to_string()))
       .unwrap())
}

fn create_error_response(status: StatusCode, message: String) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::builder()
       .status(status)
       .body(message.into())
       .unwrap())
}

async fn get_questions(pool: Arc<PgPool>, product_id: i32, page: i32, count: i32) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    let limit = (page * count) as i64;

    let row = sqlx::query!(
        r#"
        SELECT
            COALESCE(
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
                ), '[]'::json
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

    let results = if let Some(row) = row {
        serde_json::from_value(row.results.into()).unwrap_or_else(|_| serde_json::Value::Array(vec![]))
    } else {
        serde_json::Value::Array(vec![])
    };

    let mut response = serde_json::Map::new();
    response.insert("product_id".to_string(), serde_json::Value::from(product_id));
    response.insert("results".to_string(), results);

    create_success_response(StatusCode::OK, serde_json::Value::Object(response))
}

async fn get_answers(pool: Arc<PgPool>, question_id: i32, page: i32, count: i32) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    let limit = (page * count) as i64;

    let row = sqlx::query!(
        r#"
        SELECT
            COALESCE(
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
                    ), '[]'::json 
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

    let results = if let Some(row) = row {
        serde_json::from_value(row.results.into()).unwrap_or_else(|_| serde_json::Value::Array(vec![]))
    } else {
        serde_json::Value::Array(vec![])
    };

    let mut response = serde_json::Map::new();
    response.insert("question_id".to_string(), serde_json::Value::from(question_id));
    response.insert("page".to_string(), serde_json::Value::from(page));
    response.insert("count".to_string(), serde_json::Value::from(count));
    response.insert("results".to_string(), results);

    create_success_response(StatusCode::OK, serde_json::Value::Object(response))
}

async fn add_question(pool: Arc<PgPool>, question_data: NewQuestion) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    let result = sqlx::query!(
        r#"
        INSERT INTO questions (product_id, body, date_written, asker_name, asker_email, reported, helpful)
        VALUES ($1, $2, NOW(), $3, $4, false, 0)
        RETURNING id;
        "#,
        question_data.product_id,
        question_data.body,
        question_data.name,
        question_data.email
    )
    .fetch_one(&*pool)
    .await;

    match result {
        Ok(row) => {
            let response = serde_json::json!({ "question_id": row.id });
            create_success_response(StatusCode::CREATED, response)
        }
        Err(e) => {
            println!("Failed to add question: {:?}", e);
            return create_error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to add question".into());
        }
    }
}

async fn add_answer(pool: Arc<PgPool>, question_id: i32, answer_data: NewAnswer) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    let result = sqlx::query!(
        r#"
        INSERT INTO answers (question_id, body, date_written, answerer_name, answerer_email, reported, helpful)
        VALUES ($1, $2, NOW(), $3, $4, false, 0)
        RETURNING id;
        "#,
        question_id,
        answer_data.body,
        answer_data.name,
        answer_data.email
    )
    .fetch_one(&*pool)
    .await;

    match result {
        Ok(row) => {
            let answer_id = row.id;

            for url in answer_data.photos {
                let _ = sqlx::query!(
                    r#"
                    INSERT INTO answer_photos (answer_id, url)
                    VALUES ($1, $2);
                    "#,
                    answer_id,
                    url
                )
                .execute(&*pool)
                .await;
            }

            let response = serde_json::json!({ "answer_id": answer_id });
            create_success_response(StatusCode::CREATED, response)
        }
        Err(e) => {
            println!("Failed to add answer: {:?}", e);
            return create_error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to add answer".into());
        }
    }
}

async fn update_question_helpful(pool: Arc<PgPool>, question_id: i32) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    let result = sqlx::query!(
        r#"
        UPDATE questions
        SET helpful = helpful + 1
        WHERE id = $1;
        "#,
        question_id
    )
    .execute(&*pool)
    .await;

    match result {
        Ok(_) => create_success_response(StatusCode::NO_CONTENT, serde_json::Value::Null),
        Err(e) => {
            println!("Failed to update question helpfulness: {:?}", e);
            return create_error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to update question helpfulness".into());
        }
    }
}

async fn update_question_report(pool: Arc<PgPool>, question_id: i32) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    let result = sqlx::query!(
        r#"
        UPDATE questions
        SET reported = true
        WHERE id = $1;
        "#,
        question_id
    )
    .execute(&*pool)
    .await;

    match result {
        Ok(_) => create_success_response(StatusCode::NO_CONTENT, serde_json::Value::Null),
        Err(e) => {
            println!("Failed to update question report: {:?}", e);
            return create_error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to update question report".into());
        }
    }
}

async fn update_answer_helpful(pool: Arc<PgPool>, answer_id: i32) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    let result = sqlx::query!(
        r#"
        UPDATE answers
        SET helpful = helpful + 1
        WHERE id = $1;
        "#,
        answer_id
    )
    .execute(&*pool)
    .await;

    match result {
        Ok(_) => create_success_response(StatusCode::NO_CONTENT, serde_json::Value::Null),
        Err(e) => {
            println!("Failed to update answer helpfulness: {:?}", e);
            return create_error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to update answer helpfulness".into());
        }
    }
}

async fn update_answer_report(pool: Arc<PgPool>, answer_id: i32) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    let result = sqlx::query!(
        r#"
        UPDATE answers
        SET reported = true
        WHERE id = $1;
        "#,
        answer_id
    )
    .execute(&*pool)
    .await;

    match result {
        Ok(_) => create_success_response(StatusCode::NO_CONTENT, serde_json::Value::Null),
        Err(e) => {
            println!("Failed to update answer report: {:?}", e);
            return create_error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to update answer report".into());
        }
    }
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
