use std::{
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};

use axum::extract::ws::Message;
use flume::WeakSender;
use songbird::{
    Config, ConnectionInfo, CoreEvent, Driver, Event, TrackEvent,
    id::{GuildId, UserId},
    tracks::{Track, TrackHandle, TrackState},
};
use tokio::sync::Mutex;

use super::{events::PlayerEvent, manager::CleanerSender};
use crate::{
    Sources,
    models::{Player as ApiPlayer, PlayerState, Track as ApiTrack, VoiceData},
    util::{decoder::decode_base64, errors::PlayerError, source::Source},
};

#[derive(Clone)]
pub struct Player {
    pub user_id: UserId,
    pub guild_id: GuildId,
    pub active: Arc<AtomicBool>,
    pub data: Arc<Mutex<ApiPlayer>>,
    websocket: WeakSender<Message>,
    cleaner: WeakSender<CleanerSender>,
    driver: Arc<Mutex<Driver>>,
    handle: Arc<Mutex<Option<TrackHandle>>>,
}

impl Player {
    pub async fn new(
        websocket: WeakSender<Message>,
        cleaner: WeakSender<CleanerSender>,
        config: Option<Config>,
        user_id: UserId,
        guild_id: GuildId,
        server_update: VoiceData,
    ) -> Result<Self, PlayerError> {
        let mut manager = Driver::new(config.unwrap_or_default());

        let connection = ConnectionInfo {
            channel_id: None,
            endpoint: server_update.endpoint.to_owned(),
            session_id: server_update.session_id.to_owned(),
            token: server_update.token.to_owned(),
            guild_id,
            user_id,
        };

        manager.connect(connection).await?;

        let player_info = ApiPlayer {
            guild_id: guild_id.0.get(),
            track: None,
            volume: 1,
            paused: false,
            state: PlayerState {
                // todo: fix this
                time: Instant::now().elapsed().as_secs(),
                position: 0,
                connected: true,
                ping: None,
            },
            voice: server_update,
            filters: serde_json::Value::Object(serde_json::Map::new()),
        };

        let driver = Arc::new(Mutex::new(manager));
        let active = Arc::new(AtomicBool::new(false));
        let data = Arc::new(Mutex::new(player_info));

        let player = Player {
            user_id,
            guild_id,
            active,
            data,
            websocket,
            cleaner,
            driver,
            handle: Arc::new(Mutex::new(None)),
        };

        let mut driver = player.driver.lock().await;

        driver.add_global_event(
            Event::Core(CoreEvent::DriverDisconnect),
            PlayerEvent {
                player: player.clone(),
                event_type: Event::Core(CoreEvent::DriverDisconnect),
            },
        );

        driver.add_global_event(
            Event::Periodic(Duration::from_secs(10), None),
            PlayerEvent {
                player: player.clone(),
                event_type: Event::Periodic(Duration::from_secs(10), None),
            },
        );

        drop(driver);

        Ok(player)
    }

    pub async fn get_raw_state(&self) -> Option<TrackState> {
        let lock = self.handle.lock().await;

        let handle = lock.clone()?;

        drop(lock);

        let state = handle.get_info().await.ok()?;

        Some(state.clone())
    }

    pub async fn update(
        &self,
        server_update: &VoiceData,
        config: Option<Config>,
    ) -> Result<(), PlayerError> {
        let connection = ConnectionInfo {
            channel_id: None,
            endpoint: server_update.endpoint.to_owned(),
            guild_id: self.guild_id,
            session_id: server_update.session_id.to_owned(),
            token: server_update.token.to_owned(),
            user_id: self.user_id,
        };

        let mut driver = self.driver.lock().await;

        if let Some(config) = config {
            driver.set_config(config);
        }

        driver.connect(connection).await?;

        drop(driver);

        let mut data = self.data.lock().await;

        data.voice = server_update.clone();

        Ok(())
    }

    pub async fn disconnect(&self) {
        self.stop().await;

        let mut driver = self.driver.lock().await;

        driver.leave();
    }

    pub async fn play(&self, encoded: String) -> Result<(), PlayerError> {
        let info = decode_base64(&encoded)?;

        let api_track = ApiTrack {
            encoded,
            info,
            plugin_info: serde_json::Value::Null,
        };

        let track = Player::stream_and_transform(&api_track).await?;

        self.stop().await;

        let mut driver = self.driver.lock().await;

        let track_handle = driver.play_only(track);

        drop(driver);

        track_handle.add_event(
            Event::Track(TrackEvent::Play),
            PlayerEvent {
                player: self.clone(),
                event_type: Event::Track(TrackEvent::Play),
            },
        )?;

        track_handle.add_event(
            Event::Track(TrackEvent::Pause),
            PlayerEvent {
                player: self.clone(),
                event_type: Event::Track(TrackEvent::Pause),
            },
        )?;

        track_handle.add_event(
            Event::Track(TrackEvent::Playable),
            PlayerEvent {
                player: self.clone(),
                event_type: Event::Track(TrackEvent::Playable),
            },
        )?;

        track_handle.add_event(
            Event::Track(TrackEvent::End),
            PlayerEvent {
                player: self.clone(),
                event_type: Event::Track(TrackEvent::End),
            },
        )?;

        track_handle.add_event(
            Event::Track(TrackEvent::Error),
            PlayerEvent {
                player: self.clone(),
                event_type: Event::Track(TrackEvent::Error),
            },
        )?;

        let mut handle = self.handle.lock().await;

        let _ = handle.insert(track_handle);

        Ok(())
    }

    pub async fn stop(&self) {
        let mut handle = self.handle.lock().await;

        if let Some(handle) = handle.take() {
            handle.stop().ok();
        }
    }

    pub async fn remove_handle(&self) {
        self.handle.lock().await.take();
    }

    pub async fn send_ws(&self, message: Message) {
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

    pub async fn delete(&self) {
        let Some(sender) = self.cleaner.upgrade() else {
            tracing::warn!(
                "Player with [GuildId: {}] [UserId: {}] tried to send a destroy message on cleaner channel that don\'t exist",
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

    async fn stream_and_transform(track: &ApiTrack) -> Result<Track, PlayerError> {
        let input = if track.info.source_name == "http" {
            Sources.http.stream(&track.info).await?
        } else {
            return Err(PlayerError::InputNotSupported);
        };

        Ok(Track::new_with_data(input, Arc::new(track.clone())))
    }
}
