use diesel::pg::PgConnection;
use diesel::r2d2::{self, ConnectionManager};
use dotenv::dotenv;
use hyper::{Body, Request, Response, Server};
use routerify::{Middleware, RequestInfo, Router};
use std::convert::Infallible;
use std::env;

#[tokio::main]
async fn main() {
    dotenv().ok();
}
