use crate::models::{NewAnswer, NewQuestion};
use crate::utils::{
    create_error_response, get_page_count, parse_query_parameters,
};

use crate::handlers::{
    get_questions, get_answers, add_question, add_answer, update_question_helpful, update_question_report, update_answer_helpful, update_answer_report
};

use hyper::{Body, Request, Response, StatusCode};
use sqlx::PgPool;
use std::sync::Arc;

use std::collections::HashMap;

pub async fn handle_request(pool: Arc<PgPool>, req: Request<Body>) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
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
