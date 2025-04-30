use std::sync::Arc;
use dashmap::DashMap;
use songbird::{Config, ConnectionInfo, CoreEvent, Driver, Event, TrackEvent};
use songbird::id::GuildId;
use songbird::input::Input;
use songbird::tracks::TrackHandle;
use crate::events::ManagerEvent;

#[derive(Clone)]
pub struct PlayerManager {
    connections: Arc<DashMap<GuildId, Driver>>,
    handles: Arc<DashMap<GuildId, TrackHandle>>
}

impl PlayerManager {
    pub fn new() -> Self {
        Self { connections: Arc::new(DashMap::new()), handles: Arc::new(DashMap::new()) }
    }

    pub fn get_connection(&self, guild_id: GuildId) -> Option<Driver> {
        self.connections.get(&guild_id).map(|data| data.clone())
    }

    pub fn create_connection(&self, guild_id: GuildId, connection: ConnectionInfo, config: Option<Config>) {
        if self.connections.contains_key(&guild_id) {
            return;
        }

        let mut driver = Driver::new(config.unwrap_or_default());

        driver.add_global_event(Event::Core(CoreEvent::DriverDisconnect), ManagerEvent {
            manager: self.clone(),
            guild_id,
            event_type: Event::Core(CoreEvent::DriverDisconnect)
        });

        driver.connect(connection);

        self.connections.insert(guild_id, driver);
    }

    pub fn delete_connection(&self, guild_id: GuildId) {
        let Some(mut driver) = self.connections.get(&guild_id) else {
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
        let Some(mut driver) = self.connections.get(&guild_id) else {
            return;
        };

        if let Some(handle) = self.handles.get(&guild_id) {
            handle.stop().await;
        };

        let handle = driver.play_input(track);

        handle.add_event(Event::Track(TrackEvent::Playable), ManagerEvent {
            manager: self.clone(),
            guild_id,
            event_type: Event::Track(TrackEvent::Playable),
        }).ok();

        handle.add_event(Event::Track(TrackEvent::End), ManagerEvent {
            manager: self.clone(),
            guild_id,
            event_type: Event::Track(TrackEvent::End),
        }).ok();

        handle.add_event(Event::Track(TrackEvent::Error), ManagerEvent {
            manager: self.clone(),
            guild_id,
            event_type: Event::Track(TrackEvent::Error),
        }).ok();

        self.handles.insert(guild_id, handle);
    }

    pub fn delete_handle(&self, guild_id: GuildId) {
        self.handles.remove(&guild_id);
    }
}