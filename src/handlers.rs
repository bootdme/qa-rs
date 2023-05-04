use crate::models::{NewAnswer, NewQuestion};
use crate::utils::{
    create_error_response, create_success_response,
};
use hyper::{Body, Response, StatusCode};
use sqlx::PgPool;
use std::sync::Arc;

pub async fn get_questions(pool: Arc<PgPool>, product_id: i32, page: i32, count: i32) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
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

pub async fn get_answers(pool: Arc<PgPool>, question_id: i32, page: i32, count: i32) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
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

pub async fn add_question(pool: Arc<PgPool>, question_data: NewQuestion) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
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
            create_error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to add question".into())
        }
    }
}

pub async fn add_answer(pool: Arc<PgPool>, question_id: i32, answer_data: NewAnswer) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
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
            create_error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to add answer".into())
        }
    }
}

pub async fn update_question_helpful(pool: Arc<PgPool>, question_id: i32) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
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
            create_error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to update question helpfulness".into())
        }
    }
}

pub async fn update_question_report(pool: Arc<PgPool>, question_id: i32) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
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
            create_error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to update question report".into())
        }
    }
}

pub async fn update_answer_helpful(pool: Arc<PgPool>, answer_id: i32) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
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
            create_error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to update answer helpfulness".into())
        }
    }
}

pub async fn update_answer_report(pool: Arc<PgPool>, answer_id: i32) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
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
            create_error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to update answer report".into())
        }
    }
}
