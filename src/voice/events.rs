use crate::models::{
    Exception, NodeMessage, Player as ApiPlayer, PlayerEvents, PlayerUpdate, Track, TrackEnd,
    TrackException, TrackStart, WebSocketClosed,
};

use async_trait::async_trait;
use axum::extract::ws::{Message, Utf8Bytes};
use flume::WeakSender;
use songbird::{
    CoreEvent, Driver, Event, EventContext, EventHandler, TrackEvent,
    events::context_data::DisconnectReason,
    id::{GuildId, UserId},
    model::CloseCode,
    tracks::{TrackHandle, TrackState},
};
use std::sync::{
    Arc, Weak,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::Mutex;

use super::{manager::CleanerSender, player::Player};

enum DataResult {
    // probably usable in future
    #[allow(dead_code)]
    Track(TrackState, Arc<Track>),
    Disconnect(i32, String),
    Empty,
}

#[derive(Clone)]
pub struct PlayerEvent {
    pub user_id: UserId,
    pub guild_id: GuildId,
    pub event: Event,
    pub active: Weak<AtomicBool>,
    pub data: Weak<Mutex<ApiPlayer>>,
    pub websocket: WeakSender<Message>,
    pub cleaner: WeakSender<CleanerSender>,
    pub driver: Weak<Mutex<Option<Driver>>>,
    pub handle: Weak<Mutex<Option<TrackHandle>>>,
}

impl PlayerEvent {
    pub fn new(event: Event, player: &Player) -> Self {
        Self {
            user_id: player.user_id,
            guild_id: player.guild_id,
            event,
            active: Arc::downgrade(&player.active),
            data: Arc::downgrade(&player.data),
            websocket: player.websocket.clone(),
            cleaner: player.cleaner.clone(),
            driver: Arc::downgrade(&player.driver),
            handle: Arc::downgrade(&player.handle),
        }
    }

    pub async fn get_track_handle(&self) -> Option<TrackHandle> {
        self.handle.upgrade()?.lock().await.clone()
    }

    pub async fn get_track_state(&self) -> Option<TrackState> {
        self.get_track_handle().await?.get_info().await.ok()
    }

    pub async fn stop(&self, stop: bool) -> Option<()> {
        let mutex = self.handle.upgrade()?;

        let handle = mutex.lock().await.take()?;

        if !stop {
            return None;
        }

        handle.stop().ok()
    }

    pub async fn disconnect(&self, stop: bool) -> Option<()> {
        if stop {
            self.stop(stop).await;
        }

        let arc = self.driver.upgrade()?;

        let mut guard = arc.lock().await;

        if let Some(driver) = guard.as_mut() {
            driver.leave();
        }

        Some(())
    }

    pub async fn destroy(&self) {
        let Some(sender) = self.cleaner.upgrade() else {
            tracing::warn!(
                "Player Event Handler with [GuildId: {}] [UserId: {}] tried to send a destroy message on cleaner channel that don\'t exist",
                self.guild_id,
                self.user_id
            );
            return;
        };

        sender
            .send_async(CleanerSender::GuildId(self.guild_id))
            .await
            .ok();
    }

    pub async fn send_to_websocket(&self, message: Message) {
        let Some(sender) = self.websocket.upgrade() else {
            tracing::warn!(
                "Player with [GuildId: {}] [UserId: {}] tried to send on a websocket message on a websocket channel that don\'t exist",
                self.guild_id,
                self.user_id
            );
            return;
        };

        sender.send_async(message).await.ok();
    }
}

#[async_trait]
impl EventHandler for PlayerEvent {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let mut data_result = DataResult::Empty;

        match ctx {
            EventContext::Track([(state, handle)]) => {
                let state = state.to_owned().clone();

                let track = handle.data::<Track>();

                data_result = DataResult::Track(state, track);
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

                data_result = DataResult::Disconnect(code, message.to_string());
            }
            _ => {}
        };

        let player_event = self.clone();

        tokio::spawn(async move {
            handle_player_event(player_event, data_result).await;
        });

        None
    }
}

async fn handle_player_event(player_event: PlayerEvent, data_result: DataResult) -> Option<()> {
    match player_event.event {
        Event::Periodic(_, _) => {
            let state = player_event.get_track_state().await?;

            let arc = player_event.data.upgrade()?;

            let mut data = arc.lock().await;

            data.state.position = state.position.as_millis() as u32;
            data.volume = state.volume as u32;

            let event = PlayerUpdate {
                guild_id: player_event.guild_id.0.get(),
                state: data.state.clone(),
            };

            drop(data);
            drop(arc);

            let serialized = serde_json::to_string(&NodeMessage::PlayerUpdate(event)).ok()?;

            player_event
                .send_to_websocket(Message::Text(Utf8Bytes::from(serialized)))
                .await;

            Some(())
        }
        Event::Track(event) => {
            let DataResult::Track(_, track) = data_result else {
                tracing::warn!("Expected DataResult::Track but got a different thing");
                return None;
            };

            match event {
                TrackEvent::Pause => {
                    let arc = player_event.data.upgrade()?;

                    let mut data = arc.lock().await;

                    data.paused = true;

                    Some(())
                }
                TrackEvent::Play => {
                    let arc = player_event.data.upgrade()?;

                    let mut data = arc.lock().await;

                    data.paused = false;

                    Some(())
                }
                TrackEvent::End => {
                    player_event
                        .active
                        .upgrade()?
                        .swap(false, Ordering::Relaxed);

                    let arc = player_event.data.upgrade()?;

                    let mut data = arc.lock().await;

                    data.track.take();
                    data.state.position = 0;

                    drop(data);
                    drop(arc);

                    player_event.stop(false).await;

                    let event = TrackEnd {
                        guild_id: player_event.guild_id.0.get(),
                        track: track.as_ref().clone(),
                        reason: String::from("Done playing the track"),
                    };

                    let serialized = serde_json::to_string(&NodeMessage::Event(
                        PlayerEvents::TrackEndEvent(event),
                    ))
                    .ok()?;

                    player_event
                        .send_to_websocket(Message::Text(Utf8Bytes::from(serialized)))
                        .await;

                    Some(())
                }
                TrackEvent::Playable => {
                    player_event
                        .active
                        .upgrade()?
                        .swap(false, Ordering::Relaxed);

                    let arc = player_event.data.upgrade()?;

                    let mut data = arc.lock().await;

                    let _ = data.track.insert(track.as_ref().clone());

                    drop(data);
                    drop(arc);

                    let event = TrackStart {
                        guild_id: player_event.guild_id.0.get(),
                        track: track.as_ref().clone(),
                    };

                    let serialized = serde_json::to_string(&NodeMessage::Event(
                        PlayerEvents::TrackStartEvent(event),
                    ))
                    .ok()?;

                    player_event
                        .send_to_websocket(Message::Text(Utf8Bytes::from(serialized)))
                        .await;

                    Some(())
                }
                TrackEvent::Error => {
                    player_event
                        .active
                        .upgrade()?
                        .swap(false, Ordering::Relaxed);

                    let arc = player_event.data.upgrade()?;

                    let mut data = arc.lock().await;

                    data.track.take();
                    data.state.position = 0;

                    drop(data);
                    drop(arc);

                    player_event.stop(false).await;

                    let event = TrackException {
                        guild_id: player_event.guild_id.0.get(),
                        track: track.as_ref().clone(),
                        exception: Exception {
                            guild_id: player_event.guild_id.0.get(),
                            message: Some(String::from(
                                "The track has encountered a runtime issue",
                            )),
                            severity: String::from("COMMON"),
                            cause: String::from("TrackEvent::Error Emitted"),
                        },
                    };

                    let serialized = serde_json::to_string(&NodeMessage::Event(
                        PlayerEvents::TrackExceptionEvent(event),
                    ))
                    .ok()?;

                    player_event
                        .send_to_websocket(Message::Text(Utf8Bytes::from(serialized)))
                        .await;

                    Some(())
                }
                _ => None,
            }
        }
        Event::Core(CoreEvent::DriverDisconnect) => {
            player_event
                .active
                .upgrade()?
                .swap(false, Ordering::Relaxed);

            player_event.disconnect(true).await;
            player_event.destroy().await;

            let DataResult::Disconnect(code, reason) = data_result else {
                tracing::warn!("Expected DataResult::Disconnect but got a different thing");
                return None;
            };

            let event = WebSocketClosed {
                guild_id: player_event.guild_id.0.get(),
                code: code as usize,
                reason,
                by_remote: code != 1000,
            };

            let serialized = serde_json::to_string(&NodeMessage::Event(
                PlayerEvents::WebSocketClosedEvent(event),
            ))
            .ok()?;

            player_event
                .send_to_websocket(Message::Text(Utf8Bytes::from(serialized)))
                .await;

            Some(())
        }
        _ => None,
    }
}
