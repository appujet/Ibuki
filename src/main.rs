use std::cell::LazyCell;
use actix::Actor;
use axum::{serve, routing, Router };
use tokio::{ net, main };
use dotenv::dotenv;
use crate::ibuki::Ibuki;

mod routes;
mod websocket;
mod ibuki;
mod events;

pub static IBUKI: LazyCell<Ibuki> = LazyCell::new(|| Ibuki::new());

#[main(flavor = "multi_thread")]
async fn main() {
    dotenv().ok();

    let app = Router::new()
        .route("/", routing::get(routes::landing))
        .route("/sessions/session_id/players/guild_id", routing::get(routes::update_player));

    let listener = net::TcpListener::bind("0.0.0.0:8080").await.unwrap();

    serve(listener, app).await.unwrap();
}
