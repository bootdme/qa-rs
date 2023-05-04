use serde::Deserialize;

#[derive(Deserialize)]
pub struct NewQuestion {
    pub body: String,
    pub name: String,
    pub email: String,
    pub product_id: i32,
}

#[derive(Deserialize)]
pub struct NewAnswer {
    pub body: String,
    pub name: String,
    pub email: String,
    pub photos: Vec<String>,
}
