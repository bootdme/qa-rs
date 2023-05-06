mod handlers;
mod models;
mod utils;
mod routes;

use dotenv::dotenv;
use hyper::{ 
    Server,
    service::{make_service_fn, service_fn},
    server::conn::AddrStream,
};

use sqlx::postgres::PgPoolOptions;
use std::{net::SocketAddr, sync::Arc};

/// The entry point for the application.
///
/// This function initializes the environment variables, creates a connection pool
/// to the database, sets up the server to listen on a specific address, and
/// starts the server.
///
/// # Errors
///
/// Returns an error if any of the following occurs:
/// - Failed to load environment variables
/// - Failed to create a connection pool to the database
/// - Failed to bind the server to the specified address
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")?;

    let pool = PgPoolOptions::new()
        .max_connections(1000)
        .idle_timeout(std::time::Duration::from_secs(30))
        .connect(&database_url)
        .await?;

    // Wrap the connection pool in an Arc for shared ownership and thread safety
    let pool = Arc::new(pool);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // Create a service factory function that handles incoming connections
    let make_svc = make_service_fn(move |_conn: &AddrStream| {
        // Clone the connection pool for each incoming connection
        let pool = pool.clone();

        // Return a service function that handles incoming requests and passes them to the router
        async { Ok::<_, hyper::Error>(service_fn(move |req| routes::handle_request(pool.clone(), req))) }
    });

    let server = Server::bind(&addr).serve(make_svc);

    println!("Listening on http://{}", addr);

    server.await?;

    Ok(())
}
