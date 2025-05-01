use crate::websocket::client::WebsocketClient;
use axum::{
    Router,
    extract::ws::{Message, Utf8Bytes},
    middleware::from_fn,
    routing, serve,
};
use dashmap::DashMap;
use dotenv::dotenv;
use models::lavalink::{Cpu, LavalinkMessage, Memory, Stats};
use songbird::id::UserId;
use std::net::SocketAddr;
use std::sync::LazyLock;
use tokio::{
    main, net,
    time::{Duration, interval},
};
use tracing::Level;
use tracing_subscriber::fmt;

mod auth;
mod events;
mod manager;
mod models;
mod routes;
mod websocket;

pub static CLIENTS: LazyLock<DashMap<UserId, WebsocketClient>> = LazyLock::new(DashMap::new);

#[main(flavor = "multi_thread")]
async fn main() {
    dotenv().ok();

    let subscriber = fmt()
        .pretty()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_target(true)
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set global logger");

    LazyLock::force(&CLIENTS);

    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(30));

        loop {
            interval.tick().await;

            for client in CLIENTS.iter_mut() {
                // todo: fix stats placeholder
                let stats = Stats {
                    players: 0,
                    playing_players: 0,
                    uptime: 0,
                    memory: Memory {
                        free: 0,
                        used: 0,
                        allocated: 0,
                        reservable: 0,
                    },
                    cpu: Cpu {
                        cores: 0,
                        system_load: 0.0,
                        lavalink_load: 0.0,
                    },
                    frame_stats: None,
                };

                let serialized = serde_json::to_string(&LavalinkMessage::Stats(stats)).unwrap();

                client
                    .send(Message::Text(Utf8Bytes::from(serialized)))
                    .await;
            }
        }
    });

    let app = Router::new()
        .route("/lavalink/v4/websocket", routing::any(routes::global::ws))
        .route(
            "/lavalink/v4/sessions/{session_id}/players/{guild_id}",
            routing::get(routes::lavalink::get_player),
        )
        .route(
            "/lavalink/v4/sessions/{session_id}/players/{guild_id}",
            routing::patch(routes::lavalink::update_player),
        )
        .route(
            "/lavalink/v4/sessions/{session_id}/players/{guild_id}",
            routing::delete(routes::lavalink::destroy_player),
        )
        .route_layer(from_fn(auth::authenticate))
        .route("/", routing::get(routes::global::landing));

    let listener = net::TcpListener::bind("0.0.0.0:8080").await.unwrap();

    serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
