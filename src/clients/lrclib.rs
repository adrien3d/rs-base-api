use log::{error, info};
use serde::{Deserialize, Serialize};

const GET_LYRICS: &str = "https://lrclib.net/api/get";

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsResponse {
    pub id: i64,
    pub name: String,
    pub track_name: String,
    pub artist_name: String,
    pub album_name: Option<String>,
    pub duration: f64,
    pub instrumental: bool,
    pub plain_lyrics: String,
    pub synced_lyrics: String,
}

pub async fn get_song_lyrics(
    artist: &str,
    song_name: &str,
) -> Result<LyricsResponse, reqwest::Error> {
    let client = reqwest::Client::new();

    let res = client
        .get(GET_LYRICS)
        .query(&[
            ("artist_name", artist),
            ("track_name", song_name),
        ])
        .send()
        .await?;

    if let Err(err) = res.error_for_status_ref() {
        log::error!(
            "LRCLIB error for artist='{}' track='{}': status={}",
            artist,
            song_name,
            res.status()
        );
        return Err(err);
    }

    res.json::<LyricsResponse>().await
}