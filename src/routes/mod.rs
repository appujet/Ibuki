use serde::Deserialize;

pub mod endpoints;
pub mod global;

#[derive(Deserialize, Debug)]
pub struct PlayerMethodsPath {
    pub session_id: u128,
    pub guild_id: u64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlayerUpdateQuery {
    pub no_replace: bool,
}

#[derive(Deserialize, Debug)]
pub struct DecodeQueryString {
    pub track: String,
}

#[derive(Deserialize, Debug)]
pub struct EncodeQueryString {
    pub identifier: String,
}
