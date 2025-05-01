use crate::CLIENTS;
use crate::manager::PlayerManager;
use crate::models::lavalink::{
    Exception, LavalinkMessage, LavalinkPlayerState, PlayerEvents, PlayerUpdate, Track, TrackEnd,
    TrackException, TrackInfo, TrackStart, TrackStuck, WebSocketClosed,
};
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
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let manager = self.manager.clone();
        let guild_id = self.guild_id;
        let event_type = self.event_type;

        let state = match ctx {
            EventContext::Track([(state, _)]) => Some((*state).clone()),
            _ => None,
        };

        tokio::spawn(async move {
            let Some(client) = CLIENTS.get(&manager.user_id) else {
                tracing::warn!(
                    "No websocket client found for [UserId: {}]. Probably a broken client?",
                    manager.user_id
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

                    let serialized =
                        serde_json::to_string(&LavalinkMessage::PlayerUpdate(event)).unwrap();

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

                    let serialized = serde_json::to_string(&LavalinkMessage::Event(
                        PlayerEvents::TrackStuckEvent(event),
                    ))
                    .unwrap();

                    client
                        .send(Message::Text(Utf8Bytes::from(serialized)))
                        .await;
                }
                Event::Track(event) => {
                    let st = state.unwrap();

                    match event {
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
                                        length: st.position.as_millis() as usize,
                                        is_stream: false,
                                        position: st.position.as_millis() as usize,
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

                            let serialized = serde_json::to_string(&LavalinkMessage::Event(
                                PlayerEvents::TrackEndEvent(event),
                            ))
                            .unwrap();

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
                                        length: st.position.as_millis() as usize,
                                        is_stream: false,
                                        position: st.position.as_millis() as usize,
                                        title: "Placeholder".into(),
                                        uri: None,
                                        artwork_url: None,
                                        isrc: None,
                                        source_name: "Placeholder".into(),
                                    },
                                    plugin_info: serde_json::Value::Null,
                                },
                            };

                            let serialized = serde_json::to_string(&LavalinkMessage::Event(
                                PlayerEvents::TrackStartEvent(event),
                            ))
                            .unwrap();

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
                                        length: st.position.as_millis() as usize,
                                        is_stream: false,
                                        position: st.position.as_millis() as usize,
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

                            let serialized = serde_json::to_string(&LavalinkMessage::Event(
                                PlayerEvents::TrackExceptionEvent(event),
                            ))
                            .unwrap();

                            client
                                .send(Message::Text(Utf8Bytes::from(serialized)))
                                .await;
                        }
                        _ => {}
                    }
                }
                Event::Core(CoreEvent::DriverDisconnect) => {
                    manager.delete_handle(guild_id);
                    manager.delete_connection(guild_id);

                    let event = WebSocketClosed {
                        guild_id: guild_id.0.get(),
                        code: 1000,
                        reason: "Driver Disconnected".into(),
                        by_remote: true,
                    };

                    let serialized = serde_json::to_string(&LavalinkMessage::Event(
                        PlayerEvents::WebSocketClosedEvent(event),
                    ))
                    .unwrap();

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
