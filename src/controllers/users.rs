use std::str::FromStr;

use crate::{
    controllers::authentication::Authenticated,
    models::users::{self, User},
    DB_NAME,
};
use actix_web::{delete, get, post, put, web, HttpResponse};
use json;
use mongodb::{
    bson::{self, doc},
    Client, Collection,
};

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
                Ok(_) => HttpResponse::Created().body(""),
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

/// Updates a user.
#[put("/{id}")]
pub async fn update_user(
    auth: Authenticated,
    client: web::Data<Client>,
    id: web::Path<String>,
    body: web::Bytes,
) -> HttpResponse {
    log::debug!("auth: {auth:?}");
    let user_id = id.into_inner();

    let json_parse_res = json::parse(std::str::from_utf8(&body).unwrap()); // return Result
    let user_in_json: json::JsonValue = match json_parse_res {
        Ok(v) => v,
        Err(e) => json::object! {"err" => e.to_string() },
    };

    match User::from_json_value(&user_in_json) {
        Some(new_user) => {
            let collection: Collection<User> =
                client.database(DB_NAME).collection(users::REPOSITORY_NAME);

            let old_user: User;
            let user_obj_id = mongodb::bson::oid::ObjectId::from_str(&user_id).unwrap();
            match collection.find_one(doc! { "_id": user_obj_id }, None).await {
                Ok(Some(user)) => old_user = user,
                Ok(None) => old_user = new_user.clone(),
                Err(err) => {
                    log::error!("No user found with email while updating: {err}");
                    old_user = new_user.clone()
                }
            }

            let filter = doc! {"_id": &old_user.clone()._id};
            let mut new_user_copy = new_user.clone();
            new_user_copy._id = old_user._id;
            let new_user_bson = bson::to_bson(&new_user_copy).unwrap();
            //let user_doc = new_user_bson.as_document().unwrap();
            let update = doc! {"$set": new_user_bson };

            //let update = doc! {"$set": {"first_name": new_user_copy.first_name}};
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
        None => HttpResponse::InternalServerError().body("User from json error"),
    }
}

/// Deletes a user.
#[delete("/{id}")]
pub async fn delete_user_by_id(
    client: web::Data<Client>,
    id: web::Path<String>,
) -> HttpResponse {
    let id = id.into_inner();
    let user_obj_id = mongodb::bson::oid::ObjectId::from_str(&id).unwrap();
    let collection: Collection<users::User> =
        client.database(DB_NAME).collection(users::REPOSITORY_NAME);
    match collection.delete_one(doc! { "_id": &user_obj_id }, None).await {
        Ok(res) => HttpResponse::Ok().body(res.deleted_count.to_string()),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}
