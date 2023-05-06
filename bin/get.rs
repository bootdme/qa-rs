use rand::Rng;
use goose::prelude::*;
use std::{time::Duration, thread::sleep};

#[tokio::main]
async fn main() -> Result<(), GooseError> {
    sleep(Duration::from_secs(5));

    let _goose_metrics = GooseAttack::initialize()?
        .register_scenario(scenario!("GetQuestions")
            .register_transaction(transaction!(get_questions))
        )
        .register_scenario(scenario!("GetAnswers")
            .register_transaction(transaction!(get_answers))
        )
        .set_default(GooseDefault::Host, "http://localhost:3000")?
        .set_default(GooseDefault::Users, 1000)?
        .set_default(GooseDefault::HatchRate, "34")?
        .set_default(GooseDefault::ReportFile, "metrics.html")?
        .set_default(GooseDefault::RunTime, 60)?
        .execute()
        .await?;

    Ok(())
}

async fn get_questions(user: &mut GooseUser) -> TransactionResult {
    let max_product_id = 1000011;
    let random_product_id = rand::thread_rng().gen_range(1..max_product_id);

    let _response = user.get(&format!("/api/v1/questions?product_id={}", random_product_id)).await?;

    Ok(())
}

async fn get_answers(user: &mut GooseUser) -> TransactionResult {
    let max_product_id = 1000011;
    let random_product_id = rand::thread_rng().gen_range(1..max_product_id);

    let _response = user.get(&format!("/api/v1/questions/{}/answers", random_product_id)).await?;

    Ok(())
}
