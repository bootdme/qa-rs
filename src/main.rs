use dotenv::dotenv;
use hyper::{Request, Response, Body, StatusCode, service::{make_service_fn, service_fn}, server::conn::AddrStream, Server};
use serde_json::json;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::sync::Mutex;

use std::{net::SocketAddr, sync::Arc};

async fn handle_request(pool: Arc<Mutex<PgPool>>, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&hyper::Method::GET, "/questions") => get_questions(pool).await,
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("Not found".into())
            .unwrap()),
    }
}

async fn get_questions(pool: Arc<Mutex<PgPool>>) -> Result<Response<Body>, hyper::Error> {
    let pool = pool.lock().await;
    let rows = sqlx::query!(
        r#"
        SELECT product_id
        FROM questions
        LIMIT 5
        "#
    )
    .fetch_all(&*pool)
    .await
    .expect("Failed to fetch data from the database");

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
    // Load environment variables from .env file
    dotenv().ok();

    // Get database URL from environment variable
    let database_url = std::env::var("DATABASE_URL")?;

    // Initialize SQLx connection pool
    let pool = PgPoolOptions::new().max_connections(5).connect(&database_url).await?;
    let pool = Arc::new(Mutex::new(pool));

    // Define the address and port the server will listen on
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // Create a service function that handles incoming connections
    let make_svc = make_service_fn(move |_conn: &AddrStream| {
        // Clone the connection pool for each incoming connection
        let pool = pool.clone();

        // Create the service function that handles incoming requests
        async { Ok::<_, hyper::Error>(service_fn(move |req| handle_request(pool.clone(), req))) }
    });

    // Create server instance with the specified address and service function
    let server = Server::bind(&addr).serve(make_svc);

    // Start the server and print the listening address
    println!("Listening on http://{}", addr);

    // Wait for the server to shut down
    server.await?;

    Ok(())
}
