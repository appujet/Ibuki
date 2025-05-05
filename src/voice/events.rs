use crate::models::{
    Exception, NodeMessage, PlayerEvents, PlayerState, PlayerUpdate, Track, TrackEnd,
    TrackException, TrackInfo, TrackStart, TrackStuck, WebSocketClosed,
};

use async_trait::async_trait;
use axum::extract::ws::{Message, Utf8Bytes};
use songbird::{CoreEvent, Event, EventContext, EventHandler, TrackEvent};
use std::sync::atomic::Ordering;

use super::player::Player;

pub struct PlayerEvent {
    pub player: Player,
    pub event_type: Event,
}

#[async_trait]
impl EventHandler for PlayerEvent {
    async fn act(&self, _: &EventContext<'_>) -> Option<Event> {
        let player = self.player.clone();
        let event_type = self.event_type;

        tokio::spawn(async move {
            // todo: fix event data by slowly adding data on placeholder values as implementation continues
            match event_type {
                Event::Periodic(_, _) => {
                    let event = PlayerUpdate {
                        guild_id: player.guild_id.0.get(),
                        state: PlayerState {
                            time: 0,
                            position: 0,
                            connected: true,
                            ping: None,
                        },
                    };

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
                Event::Delayed(duration) => {
                    let event = TrackStuck {
                        guild_id: player.guild_id.0.get(),
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

                    let Ok(serialized) = serde_json::to_string(&NodeMessage::Event(
                        PlayerEvents::TrackStuckEvent(event),
                    )) else {
                        tracing::warn!(
                            "Serde player update encoding failed. [UserId: {}] [GuildId: {}] [Event: {}]",
                            player.user_id,
                            player.guild_id,
                            "PlayerStuck"
                        );
                        return;
                    };

                    player
                        .send_ws(Message::Text(Utf8Bytes::from(serialized)))
                        .await;
                }
                Event::Track(event) => match event {
                    TrackEvent::End => {
                        player.active.swap(false, Ordering::Relaxed);

                        player.stop().await;

                        let event = TrackEnd {
                            guild_id: player.guild_id.0.get(),
                            track: Track {
                                encoded: "Placeholder".into(),
                                info: TrackInfo {
                                    identifier: "Placeholder".into(),
                                    is_seekable: true,
                                    author: "Placeholder".into(),
                                    length: 0,
                                    is_stream: false,
                                    position: 0,
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

                        let event = TrackStart {
                            guild_id: player.guild_id.0.get(),
                            track: Track {
                                encoded: "Placeholder".into(),
                                info: TrackInfo {
                                    identifier: "Placeholder".into(),
                                    is_seekable: true,
                                    author: "Placeholder".into(),
                                    length: 0,
                                    is_stream: false,
                                    position: 0,
                                    title: "Placeholder".into(),
                                    uri: None,
                                    artwork_url: None,
                                    isrc: None,
                                    source_name: "Placeholder".into(),
                                },
                                plugin_info: serde_json::Value::Null,
                            },
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

                        let event = TrackException {
                            guild_id: player.guild_id.0.get(),
                            track: Track {
                                encoded: "Placeholder".into(),
                                info: TrackInfo {
                                    identifier: "Placeholder".into(),
                                    is_seekable: true,
                                    author: "Placeholder".into(),
                                    length: 0,
                                    is_stream: false,
                                    position: 0,
                                    title: "Placeholder".into(),
                                    uri: None,
                                    artwork_url: None,
                                    isrc: None,
                                    source_name: "Placeholder".into(),
                                },
                                plugin_info: serde_json::Value::Null,
                            },
                            exception: Exception {
                                guild_id: player.guild_id.0.get(),
                                message: None,
                                severity: "Placeholder".into(),
                                cause: "Placeholder".into(),
                            },
                        };

                        let Ok(serialized) = serde_json::to_string(&NodeMessage::Event(
                            PlayerEvents::TrackExceptionEvent(event),
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
                },
                Event::Core(CoreEvent::DriverDisconnect) => {
                    player.active.swap(false, Ordering::Relaxed);

                    player.disconnect().await;
                    player.delete().await;

                    let event = WebSocketClosed {
                        guild_id: player.guild_id.0.get(),
                        code: 1000,
                        reason: "Driver Disconnected".into(),
                        by_remote: true,
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
