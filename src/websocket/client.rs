use axum::extract::ws::WebSocket;
use songbird::id::GuildId;

pub struct WebsocketClient {
    pub guild_id: GuildId,
    websocket: WebSocket,
}

impl WebsocketClient {
    pub fn new(guild_id: GuildId, websocket: WebSocket) -> Self {
        Self { guild_id, websocket }
    }
}