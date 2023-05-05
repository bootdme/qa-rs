use rand::Rng;
use goose::prelude::*;

#[tokio::main]
async fn main() -> Result<(), GooseError> {
    let _goose_metrics = GooseAttack::initialize()?
        .register_scenario(scenario!("GetQuestions")
            .register_transaction(transaction!(get_questions))
        )
        .register_scenario(scenario!("GetAnswers")
            .register_transaction(transaction!(get_answers))
        )
        .set_default(GooseDefault::Host, "http://localhost:3000")?
        .set_default(GooseDefault::Users, 100)?
        .set_default(GooseDefault::StartupTime, 5)?
        .set_default(GooseDefault::RunTime, 105)?
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
