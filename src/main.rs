#![recursion_limit = "256"]

use crate::ws::client::WebsocketClient;
use axum::{
    Router,
    extract::ws::{Message, Utf8Bytes},
    middleware::from_fn,
    routing, serve,
};
use bytesize::ByteSize;
use cap::Cap;
use dashmap::DashMap;
use dlmalloc::GlobalDlmalloc;
use dotenv::dotenv;
use models::{ApiCpu, ApiMemory, ApiNodeMessage, ApiStats};
use reqwest::{Client, ClientBuilder};
use songbird::{driver::Scheduler, id::UserId};
use source::{deezer::source::Deezer, http::Http, youtube::Youtube};
use std::sync::LazyLock;
use std::{env::set_var, net::SocketAddr};
use tokio::{
    main, net,
    task::JoinSet,
    time::{Duration, Instant, interval},
};
use tower::ServiceBuilder;
use tracing::Level;
use tracing_subscriber::fmt;
use util::{
    headers::generate_headers,
    source::{Source, Sources},
};

mod constants;
mod middlewares;
mod models;
mod routes;
mod source;
mod util;
mod voice;
mod ws;

#[global_allocator]
static ALLOCATOR: Cap<GlobalDlmalloc> =
    Cap::new(GlobalDlmalloc, ByteSize::mb(128).as_u64() as usize);
#[allow(non_upper_case_globals)]
pub static Scheduler: LazyLock<Scheduler> = LazyLock::new(Scheduler::default);
#[allow(non_upper_case_globals)]
pub static Clients: LazyLock<DashMap<UserId, WebsocketClient>> = LazyLock::new(DashMap::new);
#[allow(non_upper_case_globals)]
pub static AvailableSources: LazyLock<DashMap<String, Sources>> = LazyLock::new(DashMap::new);
#[allow(non_upper_case_globals)]
pub static Start: LazyLock<Instant> = LazyLock::new(Instant::now);
#[allow(non_upper_case_globals)]
pub static Reqwest: LazyLock<Client> = LazyLock::new(|| {
    let builder = ClientBuilder::new().default_headers(generate_headers().unwrap());
    builder.build().expect("Failed to create reqwest client")
});

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
        .with_max_level(Level::DEBUG)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set global logger");

    LazyLock::force(&Clients);
    LazyLock::force(&AvailableSources);
    LazyLock::force(&Start);
    LazyLock::force(&Reqwest);

    {
        let src_name = String::from("Youtube");

        AvailableSources.insert(
            src_name.to_lowercase(),
            Sources::Youtube(Youtube::new(Some(Reqwest.clone()))),
        );

        tracing::info!("Registered [{}] into sources list", src_name);
    }

    {
        let src_name = String::from("Deezer");
        let client = Deezer::new(Some(Reqwest.clone()));

        client.init().await;

        AvailableSources.insert(src_name.to_lowercase(), Sources::Deezer(client));

        tracing::info!("Registered [{}] into sources list", src_name);
    }

    {
        let src_name = String::from("HTTP");

        AvailableSources.insert(
            src_name.to_lowercase(),
            Sources::Http(Http::new(Some(Reqwest.clone()))),
        );

        tracing::info!("Registered [{}] into sources list", src_name);
    }

    let mut stat = perf_monitor::cpu::ProcessStat::cur().unwrap();
    let cores = perf_monitor::cpu::processor_numbers().unwrap();

    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(30));

        loop {
            interval.tick().await;

            let Ok(process_memory_info) = perf_monitor::mem::get_process_memory_info() else {
                continue;
            };

            let Ok(usage) = stat.cpu() else {
                continue;
            };

            let used = ALLOCATOR.allocated() as u64;
            let free = ALLOCATOR.remaining() as u64;
            let limit = ALLOCATOR.limit() as u64;

            tracing::info!(
                "Memory Usage: (Heap => [Used: {:.2}] [Free: {:.2}] [Limit: {:.2}]) (RSS => [{:.2}]) (VM => [{:.2}])",
                ByteSize::b(used).display().si(),
                ByteSize::b(free).display().si(),
                ByteSize::b(limit).display().si(),
                ByteSize::b(process_memory_info.resident_set_size)
                    .display()
                    .si(),
                ByteSize::b(process_memory_info.virtual_memory_size)
                    .display()
                    .si(),
            );

            let stats = ApiStats {
                players: Scheduler.total_tasks() as u32,
                playing_players: Scheduler.live_tasks() as u32,
                uptime: Start.elapsed().as_millis() as u64,
                // todo: api memory is wip
                memory: ApiMemory {
                    free,
                    used,
                    allocated: process_memory_info.resident_set_size,
                    reservable: process_memory_info.virtual_memory_size,
                },
                // todo: get actual system load later
                cpu: ApiCpu {
                    cores: cores as u32,
                    system_load: usage,
                    lavalink_load: usage,
                },
                frame_stats: None,
            };

            let serialized =
                serde_json::to_string(&ApiNodeMessage::Stats(Box::new(stats))).unwrap();

            let set = Clients
                .iter()
                .map(|client| {
                    let clone = serialized.clone();
                    async move {
                        let _ = client.send(Message::Text(Utf8Bytes::from(clone))).await;
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
                .layer(from_fn(middlewares::auth::authenticate))
                .layer(from_fn(middlewares::log::request)),
        )
        .route("/", routing::get(routes::global::landing));

    let listener = net::TcpListener::bind("0.0.0.0:8080").await.unwrap();

    tracing::info!("Server is bound to {}", listener.local_addr().unwrap());

    serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .ok();
}
