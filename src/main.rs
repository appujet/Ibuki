use std::cell::LazyCell;
use actix::Actor;
use axum::{serve, routing, Router};
use dashmap::DashMap;
use tokio::{net, main };
use dotenv::dotenv;
use songbird::model::id::GuildId;
use crate::manager::PlayerManager;

mod routes;
mod websocket;
mod manager;
mod events;

// todo: should be websocket client and session id here
pub static PLAYERS: LazyCell<DashMap<GuildId, PlayerManager>> = LazyCell::new(|| DashMap::new());

#[main(flavor = "multi_thread")]
async fn main() {
    dotenv().ok();

    let app = Router::new()
        .route("/", routing::get(routes::landing))
        .route("/sessions/session_id/players/guild_id", routing::get(routes::update_player));

    let listener = net::TcpListener::bind("0.0.0.0:8080").await.unwrap();

    serve(listener, app).await.unwrap();
}
