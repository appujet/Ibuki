use super::{DecodeQueryString, EncodeQueryString};
use crate::util::decoder::decode_base64;
use axum::{body::Body, extract::Query, response::Response};
use crate::util::errors::EndpointError;

pub async fn get_player() {}

pub async fn update_player() {}

pub async fn destroy_player() {}

pub async fn decode(query: Query<DecodeQueryString>) -> Result<Response<Body>, EndpointError<'static>> {
    let track = decode_base64(&query.track)?;
    
    Ok(Response::new(Body::from(serde_json::to_string_pretty(&track).unwrap())))
}

pub async fn encode(query: Query<EncodeQueryString>) {}
