mod configuration;
mod controllers;
mod models;
mod store;
#[cfg(test)]
mod test;

use actix_web::{get, web, App, HttpResponse, HttpServer};

use mongodb::{bson::doc, options::IndexOptions, Client, Collection, IndexModel};

const DB_NAME: &str = "base-api";


/// Gets the user with the supplied email.
#[get("/{email}")]
async fn get_user_by_email(client: web::Data<Client>, email: web::Path<String>) -> HttpResponse {
    let email = email.into_inner();
    let collection: Collection<models::users::User> = client.database(DB_NAME).collection(models::users::REPOSITORY_NAME);
    match collection
        .find_one(doc! { "email": &email }, None)
        .await
    {
        Ok(Some(user)) => HttpResponse::Ok().json(user),
        Ok(None) => {
            HttpResponse::NotFound().body(format!("No user found with email {email}"))
        }
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

/// Creates an index on the "email" field to force the values to be unique.
async fn create_email_index(client: &Client) {
    let options = IndexOptions::builder().unique(true).build();
    let model = IndexModel::builder()
        .keys(doc! { "email": 1 })
        .options(options)
        .build();
    client
        .database(DB_NAME)
        .collection::<models::users::User>(models::users::REPOSITORY_NAME)
        .create_index(model, None)
        .await
        .expect("creating an index should succeed");
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let uri = std::env::var("MONGODB_URI").unwrap_or_else(|_| "mongodb://localhost:27017".into());

    let client = Client::with_uri_str(uri).await.expect("failed to connect");
    create_email_index(&client).await;
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(client.clone()))
            .service(web::scope("/users")
                .service(get_user_by_email)
                .service(controllers::users::create_user)
            )
            //.service(authentication)
    }).bind(("127.0.0.1", 8080))?.run().await
}