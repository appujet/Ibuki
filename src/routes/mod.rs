use serde::Deserialize;

pub mod endpoints;
pub mod global;

#[derive(Deserialize, Debug)]
pub struct DecodeQueryString {
    pub track: String,
}

#[derive(Deserialize, Debug)]
pub struct EncodeQueryString {
    #[allow(dead_code)]
    pub identifier: String,
}
