mod configuration;
mod controllers;
mod middlewares;
mod models;
mod services;
mod store;
#[cfg(test)]
mod test;
mod websocket;

use actix_cors::Cors;
use actix_identity::IdentityMiddleware;
use actix_web::{http, middleware, web, App, HttpServer};
use argon2::Config;
use mongodb::{bson::oid::ObjectId, Client};
use services::ntp::Ntp;
use tokio::sync::broadcast;
use tokio::sync::broadcast::Sender;

use crate::{
    controllers::authentication::AuthState,
    middlewares::authorization::AuthenticateMiddlewareFactory,
    models::users::{self, User},
    services::ntp,
};

const DB_NAME: &str = "base-api";

/// The maximum size of a package the server will accept.
pub const MAX_FRAME_SIZE: usize = 250_000_000; // 250Mb

pub struct ProgramAppState {
    /// A Network Time Protocol used as a time source.
    pub ntp: Ntp,
    /// MongoDB client
    pub mongo_db_client: Client,
    /// A channel for messages to the UI.
    pub ui_sender_channel: Sender<Vec<u8>>,
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Start NTP and define the timestamp format
    let ntp = ntp::Ntp::new();
    let instant: String = ntp
        .current_time()
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, false);
    log::info!("NTP Time is:{instant}");

    let uri = std::env::var("MONGODB_URI").unwrap_or_else(|_| "mongodb://localhost:27017".into());
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse()
        .unwrap();

    log::info!("Connecting to DB");
    let mongo_db_client = Client::with_uri_str(uri).await.expect("failed to connect");
    models::users::create_email_index(&mongo_db_client, DB_NAME).await;

    log::info!("Server starting on port: {}", port);

    let collection = mongo_db_client
        .database(DB_NAME)
        .collection(users::REPOSITORY_NAME);

    let salt = std::env::var("SECRET_KEY").unwrap_or_else(|_| "0123".repeat(16));
    
    let hashed_password =
        argon2::hash_encoded("password".as_bytes(), salt.as_bytes(), &Config::original()).unwrap();
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

    let auth_data = AuthState {
        mongo_db: mongo_db_client.clone(),
        admin_user: Some(ObjectId::new()),
    };
    let (ui_sender_channel, _) = broadcast::channel(32);
    let app_state = web::Data::new(ProgramAppState {
        ntp,
        mongo_db_client,
        ui_sender_channel,
    });

    let time_thread = app_state.ntp.start_time_thread(app_state.clone());

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(["DELETE", "GET", "POST", "PUT"])
            .allowed_headers([http::header::AUTHORIZATION, http::header::ACCEPT])
            .allowed_header(http::header::CONTENT_TYPE)
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .app_data(web::PayloadConfig::new(MAX_FRAME_SIZE))
            .app_data(web::JsonConfig::default().limit(MAX_FRAME_SIZE))
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .wrap(cors)
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
            .service(web::resource("/ws").route(web::get().to(websocket::handle_ws)))
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await?;

    if let Err(error) = time_thread.stop().join() {
        log::error!("Failed to stop time thread: {error:?}");
    }

    Ok(())
}
