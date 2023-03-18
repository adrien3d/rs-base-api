use actix_web::{post, web, HttpResponse};
use mongodb::{Client};
use crate::models::users;

const DB_NAME: &str = "base-api";

/// Adds a new user to the "users" collection in the database.
#[post("/")]
pub async fn create_user(client: web::Data<Client>, req_user: web::Json<users::User>) -> HttpResponse {
    let collection = client.database(DB_NAME).collection(users::REPOSITORY_NAME);
    let result = collection.insert_one(req_user, None).await;
    match result {
        Ok(_) => HttpResponse::Ok().body("user added"),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}