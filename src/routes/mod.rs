use serde::Deserialize;

pub mod endpoints;
pub mod global;

#[derive(Deserialize)]
pub struct DecodeQueryString {
    pub track: String,
}

#[derive(Deserialize)]
pub struct EncodeQueryString {
    pub identifier: String,
}
