use crate::{
    controllers::authentication::{AppState, Authenticated},
    models::users,
    DB_NAME,
};
use actix_web::{delete, get, post, put, web, HttpResponse};
use mongodb::{bson::doc, Client, Collection};

/// Adds a new user to the "users" collection in the database.
#[post("/")]
pub async fn create_user(
    client: web::Data<Client>,
    req_user: web::Json<users::User>,
) -> HttpResponse {
    let collection = client.database(DB_NAME).collection(users::REPOSITORY_NAME);
    let result = collection.insert_one(req_user, None).await;
    match result {
        Ok(_) => HttpResponse::Ok().body(""),
        Err(err) => {
            println!("{}", err);
            HttpResponse::InternalServerError().body("")
        }
    }
}

/// Gets the user with the supplied email.
#[get("/{email}")]
pub async fn get_user_by_email(
    //app_data: AppState,
    client: web::Data<Client>,
    email: web::Path<String>,
    auth: Authenticated,
) -> HttpResponse {
    log::debug!("auth: {auth:?}");
    let email = email.into_inner();
    let collection: Collection<users::User> =
        client.database(DB_NAME).collection(users::REPOSITORY_NAME);
    match collection.find_one(doc! { "email": &email }, None).await {
        Ok(Some(user)) => HttpResponse::Ok().json(user),
        Ok(None) => HttpResponse::NotFound().body(format!("No user found with email {email}")),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

/// Updates a user
#[put("/")]
pub async fn update_user(
    client: web::Data<Client>,
    req_user: web::Json<users::User>,
) -> HttpResponse {
    let collection = client.database(DB_NAME).collection(users::REPOSITORY_NAME);

    let result = collection.insert_one(req_user, None).await;
    match result {
        Ok(_) => HttpResponse::Ok().body(""),
        Err(err) => {
            println!("{}", err);
            HttpResponse::InternalServerError().body("")
        }
    }
}

/// Deletes a user
#[delete("/{email}")]
pub async fn delete_user_by_email(
    client: web::Data<Client>,
    email: web::Path<String>,
) -> HttpResponse {
    let email = email.into_inner();
    let collection: Collection<users::User> =
        client.database(DB_NAME).collection(users::REPOSITORY_NAME);
    match collection.find_one(doc! { "email": &email }, None).await {
        Ok(Some(user)) => HttpResponse::Ok().json(user),
        Ok(None) => HttpResponse::NotFound().body(format!("No user found with email {email}")),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}
