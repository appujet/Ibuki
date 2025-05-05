use std::sync::{Arc, atomic::AtomicBool};

use axum::extract::ws::Message;
use songbird::{
    Config, ConnectionInfo, CoreEvent, Driver, Event, TrackEvent,
    id::{GuildId, UserId},
    tracks::{Track, TrackHandle},
};
use tokio::sync::Mutex;

use super::events::PlayerEvent;
use crate::voice::manager::PlayerManager;
use crate::{
    Clients, Sources,
    models::{TrackInfo, VoiceData},
    util::{decoder::decode_base64, errors::PlayerError, source::Source},
};

#[derive(Clone)]
pub struct Player {
    pub user_id: UserId,
    pub guild_id: GuildId,
    pub active: Arc<AtomicBool>,
    manager: PlayerManager,
    driver: Arc<Mutex<Driver>>,
    handle: Arc<Mutex<Option<TrackHandle>>>,
}

impl Player {
    pub async fn new(
        manager: PlayerManager,
        user_id: UserId,
        guild_id: GuildId,
    ) -> Result<Self, PlayerError> {
        let driver = Arc::new(Mutex::new(Driver::new(Default::default())));
        let active = Arc::new(AtomicBool::new(false));

        let player = Player {
            user_id,
            guild_id,
            active,
            manager,
            driver,
            handle: Arc::new(Mutex::new(None)),
        };

        player.driver.lock().await.add_global_event(
            Event::Core(CoreEvent::DriverDisconnect),
            PlayerEvent {
                player: player.clone(),
                event_type: Event::Core(CoreEvent::DriverDisconnect),
            },
        );

        Ok(player)
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

        let mut driver = self.driver.lock().await;

        if let Some(config) = config {
            driver.set_config(config);
        }

        driver.connect(connection).await?;

        Ok(())
    }

    pub async fn disconnect(&self) {
        self.stop().await;

        let mut driver = self.driver.lock().await;

        driver.leave();
    }

    pub async fn play(&self, encoded: String) -> Result<(), PlayerError> {
        let track = Player::stream_and_transform(&decode_base64(&encoded)?).await?;

        self.stop().await;

        let mut driver = self.driver.lock().await;

        let track_handle = driver.play_only(track);

        drop(driver);

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

    pub async fn send_ws(&self, message: Message) {
        let Some(client) = Clients.get(&self.user_id) else {
            tracing::warn!(
                "Player with [GuildId: {}] [UserId: {}] tried to send on a websocket client that don't exist",
                self.guild_id,
                self.user_id
            );
            return;
        };

        client.send(message).await;
    }

    pub async fn delete(&self) {
        let _ = self.manager.destroy_player(self.guild_id).await;
    }

    async fn stream_and_transform(track: &TrackInfo) -> Result<Track, PlayerError> {
        let input = if track.source_name == "http" {
            Sources.http.stream(track).await?
        } else {
            return Err(PlayerError::InputNotSupported);
        };

        Ok(Track::new(input))
    }
}
