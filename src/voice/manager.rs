use crate::models::ApiVoiceData;
use crate::util::errors::PlayerManagerError;

use super::player::Player;

use axum::extract::ws::Message;
use dashmap::DashMap;
use dashmap::mapref::one::Ref;
use flume::{Sender, WeakSender, unbounded};
use songbird::Config;
use songbird::id::{GuildId, UserId};
use std::sync::Arc;

pub enum CleanerSender {
    GuildId(GuildId),
    Destroy,
}

pub struct PlayerManager {
    pub user_id: UserId,
    pub players: Arc<DashMap<GuildId, Player>>,
    cleaner: Sender<CleanerSender>,
    websocket: WeakSender<Message>,
}

impl PlayerManager {
    pub fn new(websocket: WeakSender<Message>, user_id: UserId) -> Self {
        let (cleaner, listener) = unbounded::<CleanerSender>();

        let manager = Self {
            user_id,
            cleaner,
            websocket,
            players: Arc::new(DashMap::new()),
        };

        let players = manager.players.clone();

        tokio::spawn(async move {
            while let Ok(data) = listener.recv_async().await {
                if let CleanerSender::GuildId(guild_id) = data {
                    players.remove(&guild_id);
                    continue;
                }
                break;
            }
        });

        manager
    }

    pub fn get_player(&self, guild_id: &GuildId) -> Option<Ref<'_, GuildId, Player>> {
        self.players.get(guild_id)
    }

    pub async fn create_player(
        &self,
        guild_id: GuildId,
        server_update: ApiVoiceData,
        config: Option<Config>,
    ) -> Result<Ref<'_, GuildId, Player>, PlayerManagerError> {
        let Some(player) = self.players.get(&guild_id) else {
            let player = Player::new(
                self.websocket.clone(),
                self.cleaner.downgrade(),
                config,
                self.user_id,
                guild_id,
                server_update,
            )
            .await?;

            self.players.insert(guild_id, player);

            return self
                .players
                .get(&guild_id)
                .ok_or(PlayerManagerError::MissingPlayer);
        };

        player.connect(&server_update, config).await?;

        Ok(player)
    }

    pub async fn disconnect_player(&self, guild_id: &GuildId) {
        let Some(player) = self.get_player(guild_id) else {
            return;
        };

        player.disconnect().await;
    }

    pub fn disconnect_all(&self) {
        self.players.clear();
    }
}

impl Drop for PlayerManager {
    fn drop(&mut self) {
        self.cleaner.send(CleanerSender::Destroy).ok();

        self.players.clear();

        tracing::info!("PlayerManager with [UserId: {}] dropped!", self.user_id);
    }
}
