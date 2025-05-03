use super::{DecodeQueryString, EncodeQueryString};
use crate::Sources;
use crate::models::{DataType, Track, TrackInfo};
use crate::util::errors::EndpointError;
use crate::util::source::Source;
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

    let track = Track {
        encoded: query.track.clone(),
        info: track,
        plugin_info: serde_json::Value::Null,
    };

    let string = serde_json::to_string_pretty(&track)?;

    // dummy response
    Ok(Response::new(Body::from(string)))
}

#[tracing::instrument]
pub async fn encode(query: Query<EncodeQueryString>) -> Result<Response<Body>, EndpointError> {
    let track: Option<TrackInfo> = {
        let mut result = None;

        if Sources.http.valid(query.identifier.clone()) {
            result = Some(Sources.http.resolve(query.identifier.clone()).await?)
        }

        result
    };

    let Some(track) = track else {
        let data = DataType::Empty(None);
        let string = serde_json::to_string_pretty(&data)?;

        return Ok(Response::new(Body::from(string)));
    };

    let encoded = encode_base64(&track)?;
    let track = Track {
        encoded,
        info: track,
        plugin_info: serde_json::Value::Null,
    };

    let data = DataType::Track(track);

    let string = serde_json::to_string_pretty(&data)?;

    return Ok(Response::new(Body::from(string)));
}
