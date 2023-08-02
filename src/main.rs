mod configuration;
mod controllers;
mod middlewares;
mod models;
mod store;
#[cfg(test)]
mod test;

use actix_identity::IdentityMiddleware;
use actix_web::{middleware, web, App, HttpServer};
use argon2::Config;
use mongodb::{bson::oid::ObjectId, Client};

use crate::{
    controllers::authentication::AppState,
    middlewares::authorization::AuthenticateMiddlewareFactory,
    models::users::{self, User},
};

const DB_NAME: &str = "base-api";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let uri = std::env::var("MONGODB_URI").unwrap_or_else(|_| "mongodb://localhost:27017".into());
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse()
        .unwrap();

    log::info!("Connecting to DB");
    let client = Client::with_uri_str(uri).await.expect("failed to connect");
    models::users::create_email_index(&client, DB_NAME).await;

    log::info!("Server starting on port: {}", port);

    let collection = client.database(DB_NAME).collection(users::REPOSITORY_NAME);

    let salt = std::env::var("SECRET_KEY").unwrap_or_else(|_| "0123".repeat(16));
    let config = Config::default();
    let hashed_password =
        argon2::hash_encoded("password".as_bytes(), salt.as_bytes(), &config).unwrap();

    let admin_user = User {
        _id: ObjectId::new(),
        first_name: "Adrien".to_string(),
        last_name: "Chapelet".to_string(),
        role: "god".to_string(),
        org_id: Some(ObjectId::new()),
        email: "adrien3d@gmail.com".to_string(),
        password: hashed_password,
    };
    let _ = collection.insert_one(admin_user, None).await;

    let auth_data = AppState {
        mongo_db: client.clone(),
        admin_user: Some(ObjectId::new()),
    };
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(client.clone()))
            .wrap(middleware::Logger::default())
            .service(controllers::authentication::authentication)
            .service(
                web::scope("/users")
                    .wrap(AuthenticateMiddlewareFactory::new(auth_data.clone()))
                    .wrap(IdentityMiddleware::default())
                    .service(controllers::users::create_user)
                    .service(controllers::users::get_user_by_email)
                    .service(controllers::users::update_user)
                    .service(controllers::users::delete_user_by_id),
            )
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}
