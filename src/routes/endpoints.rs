use std::num::NonZeroU64;
use std::ops::ControlFlow;

use super::{DecodeQueryString, EncodeQueryString, PlayerMethodsPath};
use crate::models::{ApiPlayerOptions, ApiTrack, ApiTrackResult};
use crate::util::converter::numbers::IbukiGuildId;
use crate::util::decoder::decode_base64;
use crate::util::errors::EndpointError;
use crate::util::source::{Source, Sources};
use crate::{AvailableSources, Clients};
use axum::Json;
use axum::extract::Path;
use axum::{body::Body, extract::Query, response::Response};
use serde_json::Value;
use songbird::id::GuildId;

#[tracing::instrument]
pub async fn get_player(
    Path(PlayerMethodsPath {
        version,
        session_id,
        guild_id,
    }): Path<PlayerMethodsPath>,
) -> Result<Response<Body>, EndpointError> {
    let client = Clients
        .iter()
        .find(|client| client.session_id == session_id)
        .ok_or(EndpointError::NotFound)?;

    let id = GuildId::from(NonZeroU64::try_from(IbukiGuildId(guild_id))?);

    let player = client
        .player_manager
        .get_player(&id)
        .ok_or(EndpointError::NotFound)?;

    let string = serde_json::to_string_pretty(&*player.data.lock().await)?;

    Ok(Response::new(Body::from(string)))
}

#[tracing::instrument]
pub async fn update_player(
    Path(PlayerMethodsPath {
        version,
        session_id,
        guild_id,
    }): Path<PlayerMethodsPath>,
    Json(update_player): Json<ApiPlayerOptions>,
) -> Result<Response<Body>, EndpointError> {
    let client = Clients
        .iter()
        .find(|client| client.session_id == session_id)
        .ok_or(EndpointError::NotFound)?;

    let id = GuildId::from(NonZeroU64::try_from(IbukiGuildId(guild_id))?);

    if client.player_manager.get_player(&id).is_none() && update_player.voice.is_none() {
        return Err(EndpointError::NotFound);
    }

    if let Some(update_voice) = update_player.voice {
        client
            .player_manager
            .create_player(id, update_voice, None)
            .await?;
    }

    let player = client
        .player_manager
        .get_player(&id)
        .ok_or(EndpointError::NotFound)?;

    if let Some(Some(encoded)) = update_player.track.map(|track| track.encoded) {
        match encoded {
            Value::Null => {
                player.stop().await;
            }
            Value::String(encoded) => {
                player.play(encoded).await?;
            }
            _ => {}
        }
    }

    let string = serde_json::to_string_pretty(&*player.data.lock().await)?;

    Ok(Response::new(Body::from(string)))
}

#[tracing::instrument]
pub async fn destroy_player(
    Path(PlayerMethodsPath {
        version,
        session_id,
        guild_id,
    }): Path<PlayerMethodsPath>,
) -> Result<Response<Body>, EndpointError> {
    let client = Clients
        .iter()
        .find(|client| client.session_id == session_id)
        .ok_or(EndpointError::NotFound)?;

    let id = GuildId::from(NonZeroU64::try_from(IbukiGuildId(guild_id))?);

    client.player_manager.disconnect_player(&id).await;

    Ok(Response::new(Body::from(())))
}

#[tracing::instrument]
pub async fn decode(query: Query<DecodeQueryString>) -> Result<Response<Body>, EndpointError> {
    let track = decode_base64(&query.track)?;

    let track = ApiTrack {
        encoded: query.track.clone(),
        info: track,
        plugin_info: serde_json::Value::Null,
    };

    let string = serde_json::to_string_pretty(&track)?;

    Ok(Response::new(Body::from(string)))
}

#[tracing::instrument]
pub async fn encode(query: Query<EncodeQueryString>) -> Result<Response<Body>, EndpointError> {
    let track: ApiTrackResult = {
        let mut result = ApiTrackResult::Empty(None);

        for source in AvailableSources.iter() {
            let mut control: ControlFlow<(), ()> = ControlFlow::Continue(());

            match source.value() {
                Sources::Youtube(src) => {
                    if src.try_search(&query.identifier).await {
                        result = src.search(&query.identifier).await?;
                        control = ControlFlow::Break(());
                    } else if src.valid_url(&query.identifier).await {
                        result = src.resolve(&query.identifier).await?;
                        control = ControlFlow::Break(());
                    }
                }
                Sources::Http(src) => {
                    if src.valid_url(&query.identifier).await {
                        result = src.resolve(&query.identifier).await?;
                        control = ControlFlow::Break(());
                    }
                }
            }

            if let ControlFlow::Break(()) = control {
                break;
            }
        }

        result
    };

    let string = serde_json::to_string_pretty(&track)?;

    return Ok(Response::new(Body::from(string)));
}
