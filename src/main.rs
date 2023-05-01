use dotenv::dotenv;
use hyper::{
    Body, Request, Response, Server, StatusCode,
    service::{make_service_fn, service_fn},
    server::conn::AddrStream,
};

use serde_json::json;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{net::SocketAddr, sync::Arc};

use std::collections::HashMap;

async fn handle_request(pool: Arc<PgPool>, req: Request<Body>) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    match (req.method(), req.uri().path()) {
        (&hyper::Method::GET, "/api/v1/questions") => {
            let query = req.uri().query();
            let params: HashMap<String, String> = query.map(|q| {
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
            }).unwrap_or_default();

            if let Some(product_id) = params.get("product_id").and_then(|v| v.parse::<i32>().ok()) {
                let page = params.get("page").and_then(|v| v.parse::<i32>().ok()).unwrap_or(1);
                let count = params.get("count").and_then(|v| v.parse::<i32>().ok()).unwrap_or(5);

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
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("Not found".into())
            .unwrap()),
    }
}

async fn get_questions(pool: Arc<PgPool>, product_id: i32, page: i32, count: i32) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    let rows = sqlx::query!(
        r#"
        SELECT product_id
        FROM questions
        LIMIT 5
        "#
    )
    .fetch_all(&*pool)
    .await
    .map_err(|e| {
        println!("Failed to fetch data from the database: {:?}", e);
        e
    })?;

    let mut data = Vec::new();
    for row in rows {
        let item = json!({
            "product_id": row.product_id,
        });
        data.push(item);
    }

    let response = serde_json::to_string(&data).unwrap();
    Ok(Response::builder()
        .header("content-type", "application/json")
        .body(response.into())
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
