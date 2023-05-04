use hyper::{Body, Response, StatusCode};
use std::collections::HashMap;

pub fn parse_query_parameters(query: Option<&str>) -> HashMap<String, String> {
    query
        .map(|q| {
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
        })
        .unwrap_or_default()
}

pub fn get_page_count(params: &HashMap<String, String>) -> (i32, i32) {
    let page = params
        .get("page")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(1);
    let count = params
        .get("count")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(5);
    (page, count)
}

pub fn create_success_response(
    status: StatusCode,
    body: serde_json::Value,
) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap())
}

pub fn create_error_response(
    status: StatusCode,
    message: String,
) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::builder()
        .status(status)
        .body(message.into())
        .unwrap())
}
