use serde::{Deserialize, Serialize};
use tokio::time::Instant;

use crate::source::deezer::ARL;

#[derive(Serialize, Debug)]
pub struct DeezerMakePlayableBody {
    #[serde(rename = "SNG_ID")]
    pub sng_id: String,
}

#[derive(Clone, Debug)]
pub struct Tokens {
    pub session_id: String,
    pub unique_id: String,
    pub check_form: String,
    pub license_token: String,
    pub expire_at: Instant,
}

#[derive(Deserialize, Debug)]
pub struct PrivateResponseError {
    #[serde(rename = "type")]
    pub cause: String,
    pub code: u16,
    pub message: String,
}

#[derive(Deserialize, Debug)]
pub struct ApiResult {
    #[serde(rename = "USER")]
    pub user: User,
    #[serde(rename = "checkForm")]
    pub check_form: String,
}

#[derive(Deserialize, Debug)]
pub struct User {
    #[serde(rename = "OPTIONS")]
    pub options: UserOptions,
}

#[derive(Deserialize, Debug)]
pub struct UserOptions {
    pub license_token: String,
}

#[derive(Deserialize, Debug)]
pub struct PrivateResponse {
    pub error: Vec<PrivateResponseError>,
    pub results: ApiResult,
}

#[derive(Deserialize, Debug)]
pub struct DeezerApiArtist {
    pub id: u32,
    pub name: String,
    pub link: String,
    #[serde(rename = "picture_medium")]
    pub thumbnail: String,
    #[serde(rename = "tracklist")]
    pub tracks: String,
}

#[derive(Deserialize, Debug)]
pub struct DeezerApiAlbum {
    pub id: u32,
    pub title: String,
    #[serde(rename = "cover_medium")]
    pub thumbnail: String,
    #[serde(rename = "tracklist")]
    pub tracks: String,
}

#[derive(Deserialize, Debug)]
pub struct DeezerApiTrack {
    pub id: u32,
    pub readable: bool,
    pub title: String,
    pub link: String,
    pub duration: u16,
    pub isrc: Option<String>,
    pub artist: DeezerApiArtist,
    pub album: DeezerApiAlbum,
}

#[derive(Deserialize, Debug)]
pub struct DeezerData<T> {
    pub data: T,
}

impl Tokens {
    pub fn create_cookie(&self) -> String {
        format!("arl={}; {}; {}", ARL, self.session_id, self.unique_id)
    }
}
