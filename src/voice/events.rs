use crate::models::{
    Exception, NodeMessage, PlayerEvents, PlayerUpdate, Track, TrackEnd, TrackException,
    TrackStart, WebSocketClosed,
};

use async_trait::async_trait;
use axum::extract::ws::{Message, Utf8Bytes};
use songbird::{
    CoreEvent, Event, EventContext, EventHandler, TrackEvent,
    events::context_data::DisconnectReason, model::CloseCode, tracks::TrackState,
};
use std::sync::{Arc, atomic::Ordering};

use super::player::Player;

enum DataResult {
    // probably usable in future
    #[allow(dead_code)]
    Track(TrackState, Arc<Track>),
    Disconnect(i32, String),
    Empty,
}

pub struct PlayerEvent {
    pub player: Player,
    pub event_type: Event,
}

#[async_trait]
impl EventHandler for PlayerEvent {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let player = self.player.clone();
        let event_type = self.event_type;

        let mut data = DataResult::Empty;

        match ctx {
            EventContext::Track([(state, handle)]) => {
                let state = state.to_owned().clone();

                let track = handle.data::<Track>();

                data = DataResult::Track(state, track);
            }
            EventContext::DriverDisconnect(info) => {
                let (code, message) = {
                    // todo: make this have the enum as reason
                    if let Some(DisconnectReason::WsClosed(Some(code))) = info.reason {
                        match code {
                            CloseCode::UnknownOpcode => (4001, "Unknown Op Code"),
                            CloseCode::InvalidPayload => (4003, "Invalid Payload"),
                            CloseCode::NotAuthenticated => (4004, "Not Authenticated"),
                            CloseCode::AuthenticationFailed => (4005, "Authentication Failed"),
                            CloseCode::AlreadyAuthenticated => (4006, "Already Authenticated"),
                            CloseCode::SessionInvalid => (4009, "Session Invalid"),
                            CloseCode::SessionTimeout => (4011, "Session Timeout"),
                            CloseCode::ServerNotFound => (4012, "Server Not Found"),
                            CloseCode::UnknownProtocol => (4012, "Unknown Protocol"),
                            CloseCode::Disconnected => (4013, "Disconnected"),
                            CloseCode::VoiceServerCrash => (4015, "Voice Server Crash"),
                            CloseCode::UnknownEncryptionMode => (4016, "Unknown Encryption Mode"),
                        }
                    } else {
                        (1000, "Graceful close")
                    }
                };

                data = DataResult::Disconnect(code, message.to_string());
            }
            _ => {}
        };

        tokio::spawn(async move {
            // todo: fix event data by slowly adding data on placeholder values as implementation continues
            match event_type {
                Event::Periodic(_, _) => {
                    let Some(state) = player.get_raw_state().await else {
                        return;
                    };

                    let mut data = player.data.lock().await;

                    data.state.position = state.position.as_millis() as u32;
                    data.volume = state.volume as u32;

                    let event = PlayerUpdate {
                        guild_id: player.guild_id.0.get(),
                        state: data.state.clone(),
                    };

                    drop(data);

                    let Ok(serialized) = serde_json::to_string(&NodeMessage::PlayerUpdate(event))
                    else {
                        tracing::warn!(
                            "Serde player update encoding failed. [UserId: {}] [GuildId: {}] [Event: {}]",
                            player.user_id,
                            player.guild_id,
                            "PlayerUpdate"
                        );
                        return;
                    };

                    player
                        .send_ws(Message::Text(Utf8Bytes::from(serialized)))
                        .await;
                }
                Event::Track(event) => {
                    let DataResult::Track(_, track) = data else {
                        tracing::warn!("Expected DataResult::Track but got a different thing");
                        return;
                    };

                    match event {
                        TrackEvent::Pause => {
                            let mut data = player.data.lock().await;
                            data.paused = true;
                            drop(data);
                        }
                        TrackEvent::Play => {
                            let mut data = player.data.lock().await;
                            data.paused = false;
                            drop(data);
                        }
                        TrackEvent::End => {
                            player.active.swap(false, Ordering::Relaxed);

                            let mut data = player.data.lock().await;

                            data.track.take();
                            data.state.position = 0;

                            drop(data);

                            player.remove_handle().await;

                            let event = TrackEnd {
                                guild_id: player.guild_id.0.get(),
                                track: track.as_ref().clone(),
                                reason: String::from("Done playing the track"),
                            };

                            let Ok(serialized) = serde_json::to_string(&NodeMessage::Event(
                                PlayerEvents::TrackEndEvent(event),
                            )) else {
                                tracing::warn!(
                                    "Serde player update encoding failed. [UserId: {}] [GuildId: {}] [Event: {}]",
                                    player.user_id,
                                    player.guild_id,
                                    "PlayerEnd"
                                );
                                return;
                            };

                            player
                                .send_ws(Message::Text(Utf8Bytes::from(serialized)))
                                .await;
                        }
                        TrackEvent::Playable => {
                            player.active.swap(true, Ordering::Relaxed);

                            let mut data = player.data.lock().await;

                            let _ = data.track.insert(track.as_ref().clone());

                            drop(data);

                            let event = TrackStart {
                                guild_id: player.guild_id.0.get(),
                                track: track.as_ref().clone(),
                            };

                            let Ok(serialized) = serde_json::to_string(&NodeMessage::Event(
                                PlayerEvents::TrackStartEvent(event),
                            )) else {
                                tracing::warn!(
                                    "Serde player update encoding failed. [UserId: {}] [GuildId: {}] [Event: {}]",
                                    player.user_id,
                                    player.guild_id,
                                    "PlayerStart"
                                );
                                return;
                            };

                            player
                                .send_ws(Message::Text(Utf8Bytes::from(serialized)))
                                .await;
                        }
                        TrackEvent::Error => {
                            player.active.swap(false, Ordering::Relaxed);

                            let mut data = player.data.lock().await;

                            data.track.take();
                            data.state.position = 0;

                            drop(data);

                            player.remove_handle().await;

                            let event = TrackException {
                                guild_id: player.guild_id.0.get(),
                                track: track.as_ref().clone(),
                                exception: Exception {
                                    guild_id: player.guild_id.0.get(),
                                    message: Some(String::from(
                                        "The track has encountered a runtime issue",
                                    )),
                                    severity: String::from("COMMON"),
                                    cause: String::from("TrackEvent::Error Emitted"),
                                },
                            };

                            let Ok(serialized) = serde_json::to_string(&NodeMessage::Event(
                                PlayerEvents::TrackExceptionEvent(event),
                            )) else {
                                tracing::warn!(
                                    "Serde player update encoding failed. [UserId: {}] [GuildId: {}] [Event: {}]",
                                    player.user_id,
                                    player.guild_id,
                                    "PlayerError"
                                );
                                return;
                            };

                            player
                                .send_ws(Message::Text(Utf8Bytes::from(serialized)))
                                .await;
                        }
                        _ => {}
                    }
                }
                Event::Core(CoreEvent::DriverDisconnect) => {
                    player.active.swap(false, Ordering::Relaxed);

                    player.disconnect().await;
                    player.delete().await;

                    let DataResult::Disconnect(code, reason) = data else {
                        tracing::warn!("Expected DataResult::Disconnect but got a different thing");
                        return;
                    };

                    let event = WebSocketClosed {
                        guild_id: player.guild_id.0.get(),
                        code: code as usize,
                        reason,
                        by_remote: code != 1000,
                    };

                    let Ok(serialized) = serde_json::to_string(&NodeMessage::Event(
                        PlayerEvents::WebSocketClosedEvent(event),
                    )) else {
                        tracing::warn!(
                            "Serde player update encoding failed. [UserId: {}] [GuildId: {}] [Event: {}]",
                            player.user_id,
                            player.guild_id,
                            "PlayerEnd"
                        );
                        return;
                    };

                    player
                        .send_ws(Message::Text(Utf8Bytes::from(serialized)))
                        .await;
                }
                _ => {}
            }
        });

        None
    }
}
