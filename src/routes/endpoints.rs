use std::num::NonZeroU64;

use super::{DecodeQueryString, EncodeQueryString, PlayerMethodsPath};
use crate::models::{DataType, Player, PlayerOptions, PlayerState, Track as IbukiTrack, VoiceData};
use crate::util::converter::numbers::IbukiGuildId;
use crate::util::decoder::decode_base64;
use crate::util::errors::EndpointError;
use crate::util::source::Source;
use crate::{Clients, Sources};
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

    let _ = client
        .player_manager
        .get_player(&id)
        .ok_or(EndpointError::NotFound)?;

    let player = Player {
        guild_id: id.0.get(),
        track: None,
        volume: 1,
        paused: false,
        state: PlayerState {
            time: 0,
            position: 0,
            connected: true,
            ping: None,
        },
        voice: VoiceData {
            token: "Placeholder".into(),
            endpoint: "Placeholder".into(),
            session_id: "Placeholder".into(),
            connected: None,
            ping: None,
        },
        filters: Value::Object(serde_json::Map::new()),
    };

    let string = serde_json::to_string_pretty(&player)?;

    Ok(Response::new(Body::from(string)))
}

#[tracing::instrument]
pub async fn update_player(
    Path(PlayerMethodsPath {
        version,
        session_id,
        guild_id,
    }): Path<PlayerMethodsPath>,
    Json(update_player): Json<PlayerOptions>,
) -> Result<Response<Body>, EndpointError> {
    tracing::info!("Got an update player request");

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

    let player = Player {
        guild_id: id.0.get(),
        track: None,
        volume: 1,
        paused: false,
        state: PlayerState {
            time: 0,
            position: 0,
            connected: true,
            ping: None,
        },
        voice: VoiceData {
            token: "Placeholder".into(),
            endpoint: "Placeholder".into(),
            session_id: "Placeholder".into(),
            connected: None,
            ping: None,
        },
        filters: Value::Object(serde_json::Map::new()),
    };

    let string = serde_json::to_string_pretty(&player)?;

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

    let track = IbukiTrack {
        encoded: query.track.clone(),
        info: track,
        plugin_info: serde_json::Value::Null,
    };

    let string = serde_json::to_string_pretty(&track)?;

    Ok(Response::new(Body::from(string)))
}

#[tracing::instrument]
pub async fn encode(query: Query<EncodeQueryString>) -> Result<Response<Body>, EndpointError> {
    let track: DataType = {
        let mut result = DataType::Empty(None);
        if Sources.youtube.valid_url(&query.identifier).await {
            result = Sources.youtube.resolve(&query.identifier).await?;
        } else if Sources.http.valid_url(&query.identifier).await {
            result = Sources.http.resolve(&query.identifier).await?;
        }

        result
    };

    tracing::info!("Got a encode request! Data: {:?}", &track);

    let string = serde_json::to_string_pretty(&track)?;

    return Ok(Response::new(Body::from(string)));
}
