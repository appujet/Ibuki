use crate::Clients;
use crate::models::{
    Exception, LavalinkMessage, LavalinkPlayerState, PlayerEvents, PlayerUpdate, Track, TrackEnd,
    TrackException, TrackInfo, TrackStart, TrackStuck, WebSocketClosed,
};
use crate::voice::manager::PlayerManager;
use async_trait::async_trait;
use axum::extract::ws::{Message, Utf8Bytes};
use songbird::id::GuildId;
use songbird::{CoreEvent, Event, EventContext, EventHandler, TrackEvent};

pub struct ManagerEvent {
    pub manager: PlayerManager,
    pub guild_id: GuildId,
    pub event_type: Event,
}

#[async_trait]
impl EventHandler for ManagerEvent {
    async fn act(&self, _: &EventContext<'_>) -> Option<Event> {
        let manager = self.manager.clone();
        let guild_id = self.guild_id;
        let event_type = self.event_type;

        tokio::spawn(async move {
            let Some(handle) = manager.get_handle(guild_id) else {
                tracing::warn!(
                    "No track handle found for [UserId: {}] [GuildId: {}]. Probably a broken client?",
                    manager.user_id,
                    guild_id
                );
                return;
            };

            let Some(client) = Clients.get(&manager.user_id) else {
                tracing::warn!(
                    "No websocket client found for [UserId: {}] [GuildId: {}]. Probably a broken client?",
                    manager.user_id,
                    guild_id
                );
                // we just clear it if this is the case
                manager.delete_connection(guild_id);
                return;
            };

            // todo: probably limit this in end and start event
            let Ok(state) = handle.get_info().await else {
                tracing::warn!(
                    "Can't fetch the track state for [UserId: {}] [GuildId: {}]. Probably a broken client?",
                    manager.user_id,
                    guild_id
                );
                return;
            };

            // todo: fix event data by slowly adding data on placeholder values as implementation continues
            match event_type {
                Event::Periodic(_, _) => {
                    let event = PlayerUpdate {
                        guild_id: guild_id.0.get(),
                        state: LavalinkPlayerState {
                            time: 0,
                            position: 0,
                            connected: true,
                            ping: None,
                        },
                    };

                    let Ok(serialized) =
                        serde_json::to_string(&LavalinkMessage::PlayerUpdate(event))
                    else {
                        tracing::warn!(
                            "Serde player update encoding failed. [UserId: {}] [GuildId: {}] [Event: {}]",
                            manager.user_id,
                            guild_id,
                            "PlayerUpdate"
                        );
                        return;
                    };

                    client
                        .send(Message::Text(Utf8Bytes::from(serialized)))
                        .await;
                }
                Event::Delayed(duration) => {
                    let event = TrackStuck {
                        guild_id: guild_id.0.get(),
                        track: Track {
                            encoded: "Placeholder".into(),
                            info: TrackInfo {
                                identifier: "Placeholder".into(),
                                is_seekable: true,
                                author: "Placeholder".into(),
                                length: 1,
                                is_stream: false,
                                position: 1,
                                title: "Placeholder".into(),
                                uri: None,
                                artwork_url: None,
                                isrc: None,
                                source_name: "Placeholder".into(),
                            },
                            plugin_info: serde_json::Value::Null,
                        },
                        threshold_ms: duration.as_millis() as usize,
                    };

                    let Ok(serialized) = serde_json::to_string(&LavalinkMessage::Event(
                        PlayerEvents::TrackStuckEvent(event),
                    )) else {
                        tracing::warn!(
                            "Serde player update encoding failed. [UserId: {}] [GuildId: {}] [Event: {}]",
                            manager.user_id,
                            guild_id,
                            "PlayerStuck"
                        );
                        return;
                    };

                    client
                        .send(Message::Text(Utf8Bytes::from(serialized)))
                        .await;
                }
                Event::Track(event) => match event {
                    TrackEvent::End => {
                        manager.delete_handle(guild_id);

                        let event = TrackEnd {
                            guild_id: guild_id.0.get(),
                            track: Track {
                                encoded: "Placeholder".into(),
                                info: TrackInfo {
                                    identifier: "Placeholder".into(),
                                    is_seekable: true,
                                    author: "Placeholder".into(),
                                    length: state.position.as_millis() as u64,
                                    is_stream: false,
                                    position: state.position.as_millis() as u64,
                                    title: "Placeholder".into(),
                                    uri: None,
                                    artwork_url: None,
                                    isrc: None,
                                    source_name: "Placeholder".into(),
                                },
                                plugin_info: serde_json::Value::Null,
                            },
                            reason: "Placeholder".into(),
                        };

                        let Ok(serialized) = serde_json::to_string(&LavalinkMessage::Event(
                            PlayerEvents::TrackEndEvent(event),
                        )) else {
                            tracing::warn!(
                                "Serde player update encoding failed. [UserId: {}] [GuildId: {}] [Event: {}]",
                                manager.user_id,
                                guild_id,
                                "PlayerEnd"
                            );
                            return;
                        };

                        client
                            .send(Message::Text(Utf8Bytes::from(serialized)))
                            .await;
                    }
                    TrackEvent::Playable => {
                        let event = TrackStart {
                            guild_id: guild_id.0.get(),
                            track: Track {
                                encoded: "Placeholder".into(),
                                info: TrackInfo {
                                    identifier: "Placeholder".into(),
                                    is_seekable: true,
                                    author: "Placeholder".into(),
                                    length: state.position.as_millis() as u64,
                                    is_stream: false,
                                    position: state.position.as_millis() as u64,
                                    title: "Placeholder".into(),
                                    uri: None,
                                    artwork_url: None,
                                    isrc: None,
                                    source_name: "Placeholder".into(),
                                },
                                plugin_info: serde_json::Value::Null,
                            },
                        };

                        let Ok(serialized) = serde_json::to_string(&LavalinkMessage::Event(
                            PlayerEvents::TrackStartEvent(event),
                        )) else {
                            tracing::warn!(
                                "Serde player update encoding failed. [UserId: {}] [GuildId: {}] [Event: {}]",
                                manager.user_id,
                                guild_id,
                                "PlayerStart"
                            );
                            return;
                        };

                        client
                            .send(Message::Text(Utf8Bytes::from(serialized)))
                            .await;
                    }
                    TrackEvent::Error => {
                        let event = TrackException {
                            guild_id: guild_id.0.get(),
                            track: Track {
                                encoded: "Placeholder".into(),
                                info: TrackInfo {
                                    identifier: "Placeholder".into(),
                                    is_seekable: true,
                                    author: "Placeholder".into(),
                                    length: state.position.as_millis() as u64,
                                    is_stream: false,
                                    position: state.position.as_millis() as u64,
                                    title: "Placeholder".into(),
                                    uri: None,
                                    artwork_url: None,
                                    isrc: None,
                                    source_name: "Placeholder".into(),
                                },
                                plugin_info: serde_json::Value::Null,
                            },
                            exception: Exception {
                                guild_id: guild_id.0.get(),
                                message: None,
                                severity: "Placeholder".into(),
                                cause: "Placeholder".into(),
                            },
                        };

                        let Ok(serialized) = serde_json::to_string(&LavalinkMessage::Event(
                            PlayerEvents::TrackExceptionEvent(event),
                        )) else {
                            tracing::warn!(
                                "Serde player update encoding failed. [UserId: {}] [GuildId: {}] [Event: {}]",
                                manager.user_id,
                                guild_id,
                                "PlayerEnd"
                            );
                            return;
                        };

                        client
                            .send(Message::Text(Utf8Bytes::from(serialized)))
                            .await;
                    }
                    _ => {}
                },
                Event::Core(CoreEvent::DriverDisconnect) => {
                    manager.delete_handle(guild_id);
                    manager.delete_connection(guild_id);

                    let event = WebSocketClosed {
                        guild_id: guild_id.0.get(),
                        code: 1000,
                        reason: "Driver Disconnected".into(),
                        by_remote: true,
                    };

                    let Ok(serialized) = serde_json::to_string(&LavalinkMessage::Event(
                        PlayerEvents::WebSocketClosedEvent(event),
                    )) else {
                        tracing::warn!(
                            "Serde player update encoding failed. [UserId: {}] [GuildId: {}] [Event: {}]",
                            manager.user_id,
                            guild_id,
                            "PlayerEnd"
                        );
                        return;
                    };

                    client
                        .send(Message::Text(Utf8Bytes::from(serialized)))
                        .await;
                }
                _ => {}
            }
        });

        None
    }
}
