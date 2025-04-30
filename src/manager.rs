use crate::events::ManagerEvent;
use dashmap::DashMap;
use songbird::id::GuildId;
use songbird::input::Input;
use songbird::tracks::TrackHandle;
use songbird::{Config, ConnectionInfo, CoreEvent, Driver, Event, TrackEvent};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Clone)]
pub struct PlayerManager {
    connections: Arc<DashMap<GuildId, Driver>>,
    handles: Arc<DashMap<GuildId, TrackHandle>>,
}

impl PlayerManager {
    pub fn new() -> Self {
        Self {
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

    pub fn create_connection(
        &self,
        guild_id: GuildId,
        connection: ConnectionInfo,
        config: Option<Config>,
    ) {
        if self.connections.contains_key(&guild_id) {
            return;
        }

        let mut driver = Driver::new(config.unwrap_or_default());

        driver.add_global_event(
            Event::Core(CoreEvent::DriverDisconnect),
            ManagerEvent {
                manager: self.clone(),
                guild_id,
                event_type: Event::Core(CoreEvent::DriverDisconnect),
            },
        );

        driver.connect(connection);

        self.connections.insert(guild_id, driver);
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

    pub async fn create_handle(&self, guild_id: GuildId, track: Input) {
        let Some(mut driver) = self.connections.get_mut(&guild_id) else {
            return;
        };

        if let Some(handle) = self.handles.get(&guild_id) {
            handle.stop().ok();
        };

        while self.handles.contains_key(&guild_id) {
            sleep(Duration::from_millis(1)).await;
        }

        let handle = driver.play_input(track);

        handle
            .add_event(
                Event::Track(TrackEvent::Playable),
                ManagerEvent {
                    manager: self.clone(),
                    guild_id,
                    event_type: Event::Track(TrackEvent::Playable),
                },
            )
            .ok();

        handle
            .add_event(
                Event::Track(TrackEvent::End),
                ManagerEvent {
                    manager: self.clone(),
                    guild_id,
                    event_type: Event::Track(TrackEvent::End),
                },
            )
            .ok();

        handle
            .add_event(
                Event::Track(TrackEvent::Error),
                ManagerEvent {
                    manager: self.clone(),
                    guild_id,
                    event_type: Event::Track(TrackEvent::Error),
                },
            )
            .ok();

        self.handles.insert(guild_id, handle);
    }

    pub fn delete_handle(&self, guild_id: GuildId) {
        self.handles.remove(&guild_id);
    }

    pub fn destroy(&self) {
        for mut connection in self.connections.iter_mut() {
            connection.leave();
        }

        self.connections.clear();
        self.handles.clear();
    }
}

impl Default for PlayerManager {
    fn default() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
            handles: Arc::new(DashMap::new()),
        }
    }
}
