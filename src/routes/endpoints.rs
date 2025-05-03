use super::{DecodeQueryString, EncodeQueryString};
use crate::util::errors::EndpointError;
use crate::util::{decoder::decode_base64, encoder::encode_base64};
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

    let decoded_and_encoded = encode_base64(&track)?;

    // debug logs, will be removed soon
    println!(
        "Original: [Length: {}]          {:?}",
        query.track.len(),
        query.track
    );
    println!(
        "Decoded & Encoded: [Length: {}] {:?}",
        decoded_and_encoded.len(),
        decoded_and_encoded,
    );

    // assert_eq!(query.track.as_str(), decoded_and_encoded.as_str());

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
