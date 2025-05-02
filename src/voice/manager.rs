use crate::util::errors::PlayerManagerError;

use super::events::ManagerEvent;
use dashmap::DashMap;
use songbird::id::{GuildId, UserId};
use songbird::tracks::{Track, TrackHandle};
use songbird::{Config, ConnectionInfo, CoreEvent, Driver, Event, TrackEvent};
use std::sync::Arc;

#[derive(Clone)]
pub struct PlayerManager {
    pub user_id: UserId,
    connections: Arc<DashMap<GuildId, Driver>>,
    handles: Arc<DashMap<GuildId, TrackHandle>>,
}

impl PlayerManager {
    pub fn new(user_id: UserId) -> Self {
        Self {
            user_id,
            connections: Arc::new(DashMap::new()),
            handles: Arc::new(DashMap::new()),
        }
    }

    pub fn get_connection_len(&self) -> usize {
        self.connections.len()
    }

    pub fn get_player_len(&self) -> usize {
        self.handles.len()
    }

    pub fn get_connection(&self, guild_id: GuildId) -> Option<Driver> {
        self.connections.get(&guild_id).map(|data| data.clone())
    }

    pub async fn create_connection(
        &self,
        guild_id: GuildId,
        connection: ConnectionInfo,
        config: Option<Config>,
    ) -> Result<Driver, PlayerManagerError> {
        let Some(driver) = self.connections.get(&guild_id) else {
            let config_or_default = config.unwrap_or_default();

            let mut driver = Driver::new(config_or_default.clone());

            driver.add_global_event(
                Event::Core(CoreEvent::DriverDisconnect),
                ManagerEvent {
                    manager: self.clone(),
                    guild_id,
                    event_type: Event::Core(CoreEvent::DriverDisconnect),
                },
            );

            driver.connect(connection.clone()).await?;

            self.connections.insert(guild_id, driver);

            return Box::pin(self.create_connection(guild_id, connection, Some(config_or_default)))
                .await;
        };

        Ok(driver.clone())
    }

    pub fn delete_connection(&self, guild_id: GuildId) {
        let Some(mut driver) = self.connections.get_mut(&guild_id) else {
            return;
        };

        driver.leave();
        driver.remove_all_global_events();

        self.connections.remove(&guild_id);
        self.handles.remove(&guild_id);
    }

    pub fn get_handle(&self, guild_id: GuildId) -> Option<TrackHandle> {
        self.handles.get(&guild_id).map(|data| data.clone())
    }

    pub async fn create_handle(
        &self,
        guild_id: GuildId,
        track: Track,
    ) -> Result<TrackHandle, PlayerManagerError> {
        let Some(mut driver) = self.connections.get_mut(&guild_id) else {
            return Err(PlayerManagerError::MissingConnection);
        };

        if let Some(handle) = self.handles.get(&guild_id) {
            handle.stop()?;
        };

        let handle = driver.play_only(track);

        handle.add_event(
            Event::Track(TrackEvent::Playable),
            ManagerEvent {
                manager: self.clone(),
                guild_id,
                event_type: Event::Track(TrackEvent::Playable),
            },
        )?;

        handle.add_event(
            Event::Track(TrackEvent::End),
            ManagerEvent {
                manager: self.clone(),
                guild_id,
                event_type: Event::Track(TrackEvent::End),
            },
        )?;

        handle.add_event(
            Event::Track(TrackEvent::Error),
            ManagerEvent {
                manager: self.clone(),
                guild_id,
                event_type: Event::Track(TrackEvent::Error),
            },
        )?;

        self.handles.insert(guild_id, handle.clone());

        Ok(handle)
    }

    pub fn delete_handle(&self, guild_id: GuildId) {
        self.handles.remove(&guild_id);
    }

    pub fn destroy(&self) {
        self.connections.retain(|_, connection| {
            connection.leave();
            connection.remove_all_global_events();
            false
        });

        self.handles.clear();
    }
}
