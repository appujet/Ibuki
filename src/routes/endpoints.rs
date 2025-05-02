use super::{DecodeQueryString, EncodeQueryString};
use crate::util::errors::EndpointError;
use crate::util::{decoder::decode_base64, encode::encode_base64};
use axum::{body::Body, extract::Query, response::Response};

#[tracing::instrument]
pub async fn get_player() {}

#[tracing::instrument]
pub async fn update_player() {}

#[tracing::instrument]
pub async fn destroy_player() {}

#[tracing::instrument]
pub async fn decode(query: Query<DecodeQueryString>) -> Result<Response<Body>, EndpointError> {
    let track = decode_base64(&query.track)?;
    // dummy response
    Ok(Response::new(Body::from(serde_json::to_string_pretty(
        &track,
    )?)))
}

#[tracing::instrument]
pub async fn encode(query: Query<EncodeQueryString>) -> Result<Response<Body>, EndpointError> {
    // dummy response
    Ok(Response::new(Body::from(query.0.identifier.clone())))
}
