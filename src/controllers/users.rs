use crate::{
    controllers::authentication::Authenticated,
    models::users::{self, User},
    DB_NAME,
};
use actix_web::{delete, get, post, put, web, HttpResponse};
use json;
use mongodb::{bson::doc, Client, Collection};

/// Adds a new user to the "users" collection in the database.
#[post("/")]
pub async fn create_user(
    auth: Authenticated,
    client: web::Data<Client>,
    body: web::Bytes,
) -> HttpResponse {
    log::debug!("auth: {auth:?}");
    let result = json::parse(std::str::from_utf8(&body).unwrap()); // return Result
    let injson: json::JsonValue = match result {
        Ok(v) => v,
        Err(e) => json::object! {"err" => e.to_string() },
    };

    match User::from_json_value(&injson) {
        Some(user) => {
            let collection = client.database(DB_NAME).collection(users::REPOSITORY_NAME);
            let result = collection.insert_one(user, None).await;
            match result {
                Ok(_) => HttpResponse::Ok().body(""),
                Err(err) => {
                    log::warn!("{}", err);
                    //TODO: Handle duplicate keys (emails)
                    HttpResponse::InternalServerError().body("")
                }
            }
        }
        None => HttpResponse::InternalServerError().body("Parsing error"),
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
        Ok(Some(user)) => HttpResponse::Ok().json(user.sanitize()),
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
