use crate::ws::client::WebsocketClient;
use axum::{
    Router,
    extract::ws::{Message, Utf8Bytes},
    middleware::from_fn,
    routing, serve,
};
use dashmap::DashMap;
use dotenv::dotenv;
use models::{Cpu, LavalinkMessage, Memory, Stats};
use songbird::id::UserId;
use std::sync::LazyLock;
use std::{env::set_var, net::SocketAddr};
use tokio::{
    main, net,
    task::JoinSet,
    time::{Duration, interval},
};
use tower::ServiceBuilder;
use tracing::Level;
use tracing_subscriber::fmt;
use util::source::SourceManager;

mod constants;
mod middlewares;
mod models;
mod routes;
mod source;
mod util;
mod voice;
mod ws;

#[allow(non_upper_case_globals)]
pub static Clients: LazyLock<DashMap<UserId, WebsocketClient>> = LazyLock::new(DashMap::new);

#[allow(non_upper_case_globals)]
pub static Sources: LazyLock<SourceManager> = LazyLock::new(SourceManager::new);

#[main(flavor = "multi_thread")]
async fn main() {
    unsafe { set_var("RUST_BACKTRACE", "1") };

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

    LazyLock::force(&Clients);

    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(30));

        loop {
            interval.tick().await;

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

            let serialized = serde_json::to_string(&LavalinkMessage::Stats(stats.clone())).unwrap();

            let set = Clients
                .iter()
                .map(|client| {
                    let clone = serialized.clone();
                    async move {
                        client.send(Message::Text(Utf8Bytes::from(clone))).await;
                    }
                })
                .collect::<JoinSet<()>>();

            set.join_all().await;
        }
    });

    let app = Router::new()
        .route("/v{version}/websocket", routing::any(routes::global::ws))
        .route(
            "/v{version}/decodetrack",
            routing::get(routes::endpoints::decode),
        )
        .route(
            "/v{version}/loadtracks",
            routing::get(routes::endpoints::encode),
        )
        .route(
            "/v{version}/sessions/{session_id}/players/{guild_id}",
            routing::get(routes::endpoints::get_player),
        )
        .route(
            "/v{version}/sessions/{session_id}/players/{guild_id}",
            routing::patch(routes::endpoints::update_player),
        )
        .route(
            "/v{version}/sessions/{session_id}/players/{guild_id}",
            routing::delete(routes::endpoints::destroy_player),
        )
        .route_layer(
            ServiceBuilder::new()
                .layer(from_fn(middlewares::version::check))
                .layer(from_fn(middlewares::auth::authenticate)),
        )
        .route("/", routing::get(routes::global::landing));

    let listener = net::TcpListener::bind("0.0.0.0:8080").await.unwrap();

    tracing::info!("Server is bound to {}", listener.local_addr().unwrap());

    serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
