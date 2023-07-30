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
    let json_parse_res = json::parse(std::str::from_utf8(&body).unwrap()); // return Result
    let user_in_json: json::JsonValue = match json_parse_res {
        Ok(v) => v,
        Err(e) => json::object! {"err" => e.to_string() },
    };

    match User::from_json_value(&user_in_json) {
        Some(user) => {
            let collection: Collection<User> =
                client.database(DB_NAME).collection(users::REPOSITORY_NAME);
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
    auth: Authenticated,
    client: web::Data<Client>,
    email: web::Path<String>,
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
    auth: Authenticated,
    client: web::Data<Client>,
    body: web::Bytes,
) -> HttpResponse {
    log::debug!("auth: {auth:?}");
    let json_parse_res = json::parse(std::str::from_utf8(&body).unwrap()); // return Result
    let user_in_json: json::JsonValue = match json_parse_res {
        Ok(v) => v,
        Err(e) => json::object! {"err" => e.to_string() },
    };

    match User::from_json_value(&user_in_json) {
        Some(new_user) => {
            let new_user_copy = new_user.clone();
            let collection: Collection<User> =
                client.database(DB_NAME).collection(users::REPOSITORY_NAME);
            let filter = doc! {"email": new_user_copy.email};
            let update = doc! {"$set": {"first_name": new_user_copy.first_name}};
            let result = collection.update_one(filter, update, None).await;
            match result {
                Ok(_) => HttpResponse::Ok().json(new_user),
                Err(err) => {
                    log::warn!("{}", err);
                    //TODO: Handle multiple fields
                    HttpResponse::InternalServerError().body("")
                }
            }
        }
        None => HttpResponse::InternalServerError().body("Parsing error"),
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
