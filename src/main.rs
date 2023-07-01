mod configuration;
mod controllers;
mod middlewares;
mod models;
mod store;
#[cfg(test)]
mod test;

use actix_web::{middleware, web, App, HttpServer};
use mongodb::Client;

use crate::middlewares::authorization::Authorization;

const DB_NAME: &str = "base-api";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let uri = std::env::var("MONGODB_URI").unwrap_or_else(|_| "mongodb://localhost:27017".into());
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse()
        .unwrap();

    println!("Connecting to DB");
    let client = Client::with_uri_str(uri).await.expect("failed to connect");
    models::users::create_email_index(&client, DB_NAME).await;

    println!("Server starting on port: {}", port);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(client.clone()))
            .service(controllers::authentication::authentication)
            .wrap(middleware::Logger::default())
            .service(
                web::scope("/users")
                    .wrap(Authorization)
                    .service(controllers::users::create_user)
                    .service(controllers::users::get_user_by_email),
            )
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}
