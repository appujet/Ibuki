use std::sync::Arc;
use dashmap::DashMap;
use songbird::{Config, ConnectionInfo, CoreEvent, Driver, Event, TrackEvent};
use songbird::error::TrackResult;
use songbird::id::GuildId;
use songbird::input::Input;
use songbird::tracks::{TrackHandle, TrackState};
use crate::events::{DriverDisconnectEvent, EndEvent, ErrorEvent, StartEvent};

#[derive(Clone)]
pub struct Ibuki {
    pub connections: Arc<DashMap<GuildId, Driver>>,
    pub handles: Arc<DashMap<GuildId, TrackHandle>>
}

impl Ibuki {
    pub fn new() -> Self {
        Self { connections: Arc::new(DashMap::new()), handles: Arc::new(DashMap::new()) }
    }

    pub fn connect_driver(&self, guild_id: GuildId, connection: ConnectionInfo) {
        let Some(mut driver) = self.connections.get(&guild_id) else {
            let _ = self.connections.insert(guild_id, Driver::new(Config::default()));
            return self.connect_driver(guild_id, connection);
        };

        driver.add_global_event(Event::Track(TrackEvent::Play), StartEvent { guild_id });
        driver.add_global_event(Event::Track(TrackEvent::End), EndEvent { guild_id });
        driver.add_global_event(Event::Track(TrackEvent::Error), ErrorEvent { guild_id });
        driver.add_global_event(Event::Core(CoreEvent::DriverDisconnect), DriverDisconnectEvent { guild_id });

        driver.connect(connection);
    }

    pub async fn get_player(&self, guild_id: GuildId) -> Option<TrackResult<TrackState>> {
        let Some(handle) = self.handles.get(&guild_id) else {
            return None;
        };

        Some(handle.get_info().await)
    }

    pub fn disconnect_driver(&self, guild_id: GuildId) {
        let Some(mut driver) = self.connections.get(&guild_id) else {
            return;
        };

        driver.leave();
        driver.remove_all_global_events();

        self.connections.remove(&guild_id);
        self.handles.remove(&guild_id);
    }

    pub async fn play_track(&self, guild_id: GuildId, track: Input) {
        let Some(mut driver) = self.connections.get(&guild_id) else {
            return;
        };

        if let Some(mut handle) = self.handles.get(&guild_id) {
            handle.stop().await;
        };

        self.handles.remove(&guild_id);

        let handle = driver.play_input(track);

        self.handles.insert(guild_id, handle);
    }

    pub async fn pause(&self, guild_id: GuildId) {
        let Some(handle) = self.handles.get(&guild_id) else {
            return;
        };

        handle.pause().await;
    }

    pub async fn stop(&self, guild_id: GuildId) {
        let Some(handle) = self.handles.get(&guild_id) else {
            return;
        };

        handle.stop().await;
    }

    pub async fn set_volume(&self, guild_id: GuildId, volume: f32) {
        let Some(handle) = self.handles.get(&guild_id) else {
            return;
        };

        handle.set_volume(volume).await;
    }
}