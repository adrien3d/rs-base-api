use std::str::FromStr;

use crate::{
    DATABASE_NAME, ProgramAppState, clients::lrclib::get_song_lyrics, controllers::authentication::Authenticated, models::users::{self, User}
};
use actix_web::{delete, get, post, put, web, HttpResponse};
use json;
use mongodb::{
    bson::{self, doc},
    Collection,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct LyricsQuery {
    artist: String,
    track: String,
}

/// Gets the user with the supplied email.
#[get("")]
pub async fn get_lyrics(
    auth: Authenticated,
    app_state: web::Data<ProgramAppState>,
    query: web::Query<LyricsQuery>,
) -> HttpResponse {
    let u = auth.get_user();
    log::debug!("user: {u:?}");
    let _ = app_state;

    match get_song_lyrics(&query.artist, &query.track).await {
        Ok(lyrics) => HttpResponse::Ok().json(lyrics),
        Err(err) => {
            log::error!(
                "Failed to fetch lyrics for artist='{}' track='{}': {}",
                query.artist,
                query.track,
                err
            );

            if let Some(status) = err.status() {
                HttpResponse::build(
                    actix_web::http::StatusCode::from_u16(status.as_u16())
                        .unwrap_or(actix_web::http::StatusCode::BAD_GATEWAY),
                )
                .body("Failed to fetch lyrics from upstream API")
            } else {
                HttpResponse::InternalServerError().body("Failed to fetch lyrics")
            }
        }
    }
}