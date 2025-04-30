use crate::websocket::client::WebsocketClient;
use axum::{Router, middleware::from_fn, routing, serve};
use dashmap::DashMap;
use dotenv::dotenv;
use std::net::SocketAddr;
use std::sync::LazyLock;
use tokio::{main, net};

mod auth;
mod events;
mod manager;
mod routes;
mod websocket;

pub static CLIENTS: LazyLock<DashMap<u128, WebsocketClient>> = LazyLock::new(DashMap::new);

#[main(flavor = "multi_thread")]
async fn main() {
    dotenv().ok();

    LazyLock::force(&CLIENTS);

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
