mod configuration;
mod controllers;
mod models;
mod store;
#[cfg(test)]
mod test;

use actix_web::{web, App, HttpServer};
use mongodb::Client;

const DB_NAME: &str = "base-api";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let uri = std::env::var("MONGODB_URI").unwrap_or_else(|_| "mongodb://localhost:27017".into());
    let port: u16 = std::env::var("PORT").unwrap_or_else(|_| "8080".into()).parse().unwrap();

    println!("Connecting to DB");
    let client = Client::with_uri_str(uri).await.expect("failed to connect");
    models::users::create_email_index(&client, DB_NAME).await;

    println!("Server starting on port: {}", port);
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(client.clone()))
            .service(web::scope("/users")
                .service(controllers::users::create_user)
                .service(controllers::users::get_user_by_email)
            )
            .service(controllers::authentication::authentication)
    }).bind(("127.0.0.1", port))?.run().await
}