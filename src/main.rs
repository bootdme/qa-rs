use std::error::Error;
use dotenv::dotenv;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load environment variables from .env file
    dotenv().ok();

    // Get database URL from environment variable
    let database_url = std::env::var("DATABASE_URL")?;

    // Initialize SQLx connection pool
    let pool = PgPoolOptions::new().max_connections(5).connect(&database_url).await?;

    // Bind to a server:port
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // Start server
    println!("Listening on http://{}", addr);

    Ok(())
}
