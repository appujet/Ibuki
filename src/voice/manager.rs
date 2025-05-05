use crate::models::VoiceData;
use crate::util::errors::PlayerManagerError;

use super::player::Player;

use dashmap::DashMap;
use dashmap::mapref::one::Ref;
use songbird::Config;
use songbird::id::{GuildId, UserId};
use std::sync::Arc;
use std::sync::atomic::Ordering;

#[derive(Clone)]
pub struct PlayerManager {
    pub user_id: UserId,
    players: Arc<DashMap<GuildId, Player>>,
}

impl PlayerManager {
    pub fn new(user_id: UserId) -> Self {
        Self {
            user_id,
            players: Arc::new(DashMap::new()),
        }
    }

    pub fn get_players_len(&self) -> usize {
        self.players.len()
    }

    pub fn get_active_players_len(&self) -> usize {
        self.players
            .iter()
            .map(|player| {
                if player.active.load(Ordering::Relaxed) {
                    1
                } else {
                    0
                }
            })
            .reduce(|acc, number| acc + number)
            .unwrap_or(0)
    }

    pub fn get_player(&self, guild_id: GuildId) -> Option<Ref<'_, GuildId, Player>> {
        self.players.get(&guild_id)
    }

    pub async fn create_player(
        &self,
        guild_id: GuildId,
        server_update: VoiceData,
        config: Option<Config>,
        exists: Option<()>,
    ) -> Result<Ref<'_, GuildId, Player>, PlayerManagerError> {
        let Some(player) = self.players.get(&guild_id) else {
            let player = Player::new(self.clone(), self.user_id, guild_id).await?;

            player.connect(&server_update, config).await?;

            self.players.insert(guild_id, player);

            return Box::pin(self.create_player(guild_id, server_update, None, Some(()))).await;
        };

        if exists.is_none() {
            player.connect(&server_update, config).await?;
        }

        Ok(player)
    }

    pub async fn destroy_player(&self, guild_id: GuildId) {
        let Some((_, player)) = self.players.remove(&guild_id) else {
            return;
        };

        player.disconnect().await;
    }

    pub async fn destroy(&self) {
        for player in self.players.iter() {
            player.stop().await;
        }

        self.players.clear();
    }
}
