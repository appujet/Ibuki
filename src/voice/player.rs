use std::{
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};

use axum::extract::ws::Message;
use flume::WeakSender;
use songbird::{
    Config, ConnectionInfo, CoreEvent, Driver, Event, TrackEvent,
    driver::Bitrate,
    id::{GuildId, UserId},
    tracks::{Track, TrackHandle, TrackState},
};
use tokio::{sync::Mutex, task};

use super::{events::PlayerEvent, manager::CleanerSender};
use crate::{
    Scheduler, Sources,
    models::{Player as ApiPlayer, PlayerState, Track as ApiTrack, VoiceData},
    util::{decoder::decode_base64, errors::PlayerError, source::Source},
};

#[derive(Clone)]
pub struct Player {
    pub user_id: UserId,
    pub guild_id: GuildId,
    pub active: Arc<AtomicBool>,
    pub data: Arc<Mutex<ApiPlayer>>,
    pub websocket: WeakSender<Message>,
    pub cleaner: WeakSender<CleanerSender>,
    pub driver: Arc<Mutex<Option<Driver>>>,
    pub handle: Arc<Mutex<Option<TrackHandle>>>,
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
        let data = ApiPlayer {
            guild_id: guild_id.0.get(),
            track: None,
            volume: 1,
            paused: false,
            state: PlayerState {
                // todo: fix this
                time: Instant::now().elapsed().as_secs(),
                position: 0,
                connected: false,
                ping: None,
            },
            voice: server_update.clone(),
            filters: serde_json::Value::Object(serde_json::Map::new()),
        };

        let active = Arc::new(AtomicBool::new(false));
        let data = Arc::new(Mutex::new(data));

        let player = Player {
            user_id,
            guild_id,
            active,
            data,
            websocket,
            cleaner,
            driver: Arc::new(Mutex::new(None)),
            handle: Arc::new(Mutex::new(None)),
        };

        player.connect(&server_update, config).await?;

        Ok(player)
    }

    pub async fn get_raw_state(&self) -> Option<TrackState> {
        let lock = self.handle.lock().await;

        let handle = lock.clone()?;

        drop(lock);

        let state = handle.get_info().await.ok()?;

        Some(state.clone())
    }

    pub async fn connect(
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

        let mut guard = self.driver.lock().await;

        if guard.is_none() {
            let config = config.unwrap_or_default().scheduler(Scheduler.to_owned());

            let mut driver = Driver::new(config.clone());

            driver.set_bitrate(Bitrate::Max);

            driver.add_global_event(
                Event::Core(CoreEvent::DriverDisconnect),
                PlayerEvent::new(Event::Core(CoreEvent::DriverDisconnect), self),
            );

            driver.add_global_event(
                Event::Periodic(Duration::from_secs(10), None),
                PlayerEvent::new(Event::Periodic(Duration::from_secs(10), None), self),
            );

            let _ = guard.insert(driver);

            drop(guard);

            return Box::pin(self.connect(server_update, Some(config))).await;
        }

        let driver = guard.as_mut().ok_or(PlayerError::MissingDriver)?;

        driver.connect(connection).await?;

        drop(guard);

        let mut guard = self.data.lock().await;

        guard.state.connected = true;
        guard.voice = server_update.clone();

        Ok(())
    }

    pub async fn disconnect(&self) {
        let mut guard = self.driver.lock().await;

        if let Some(driver) = guard.take().as_mut() {
            driver.stop();
            driver.leave();
        }

        drop(guard);

        let mut guard = self.data.lock().await;

        guard.state.connected = false;
    }

    pub async fn play(&self, encoded: String) -> Result<(), PlayerError> {
        let info = decode_base64(&encoded)?;

        let api_track = ApiTrack {
            encoded,
            info,
            plugin_info: serde_json::Value::Null,
        };

        let track = Player::stream_and_transform(&api_track).await?;

        let mut guard = self.driver.lock().await;

        let driver = guard.as_mut().ok_or(PlayerError::MissingDriver)?;

        driver.stop();

        let track_handle = driver.play_only(track);

        drop(guard);

        track_handle.add_event(
            Event::Track(TrackEvent::Play),
            PlayerEvent::new(Event::Track(TrackEvent::Play), self),
        )?;

        track_handle.add_event(
            Event::Track(TrackEvent::Pause),
            PlayerEvent::new(Event::Track(TrackEvent::Pause), self),
        )?;

        track_handle.add_event(
            Event::Track(TrackEvent::Playable),
            PlayerEvent::new(Event::Track(TrackEvent::Playable), self),
        )?;

        track_handle.add_event(
            Event::Track(TrackEvent::End),
            PlayerEvent::new(Event::Track(TrackEvent::End), self),
        )?;

        track_handle.add_event(
            Event::Track(TrackEvent::Error),
            PlayerEvent::new(Event::Track(TrackEvent::Error), self),
        )?;

        let mut handle = self.handle.lock().await;

        let _ = handle.insert(track_handle);

        Ok(())
    }

    pub async fn stop(&self) {
        let mut guard = self.driver.lock().await;

        if let Some(driver) = guard.as_mut() {
            driver.stop();
        }
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

impl Drop for Player {
    fn drop(&mut self) {
        let arc_driver = self.driver.clone();

        task::block_in_place(move || {
            if let Some(driver) = arc_driver.blocking_lock().take() {
                drop(driver);
            }
        });

        let arc_handle = self.handle.clone();

        task::block_in_place(move || {
            if let Some(handle) = arc_handle.blocking_lock().take() {
                drop(handle);
            }
        });

        tracing::info!(
            "Player with [GuildId: {}] [UserId: {}] dropped!",
            self.guild_id,
            self.user_id
        );
    }
}
