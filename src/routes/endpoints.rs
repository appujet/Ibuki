use super::{DecodeQueryString, EncodeQueryString};
use crate::util::decoder::decode_base64;
use axum::{body::Body, extract::Query, response::Response};

pub async fn get_player() {}

pub async fn update_player() {}

pub async fn destroy_player() {}

pub async fn decode(query: Query<DecodeQueryString>) -> Response<Body> {
    let track = decode_base64(&query.track);

    Response::builder()
        .body(Body::from(serde_json::to_string_pretty(&track).unwrap()))
        .unwrap()
}

pub async fn encode(query: Query<EncodeQueryString>) {}
