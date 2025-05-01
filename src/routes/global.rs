use crate::ws::client::{
    WebsocketRequestData, handle_websocket_upgrade_error, handle_websocket_upgrade_request,
};
use axum::body::Body;
use axum::extract::{ConnectInfo, WebSocketUpgrade};
use axum::http::{HeaderMap, Response, StatusCode};
use songbird::id::UserId;
use std::net::SocketAddr;
use std::num::NonZeroU64;

pub async fn landing() -> String {
    String::from("Hello World")
}

// todo: unwrap galore fix soon:tm:
pub async fn ws(
    websocket_upgrade: WebSocketUpgrade,
    headers: HeaderMap,
    connection: ConnectInfo<SocketAddr>,
) -> Response<Body> {
    let Some(user_agent) = headers.get("User-Agent").map(|data| data.to_str().unwrap()) else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from("Missing User-Agent in headers"))
            .unwrap();
    };

    let Some(user_id) = headers
        .get("User-Id")
        .map(|data| data.to_str().unwrap().parse::<u64>().unwrap())
    else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from("Missing User-Id in headers"))
            .unwrap();
    };

    let request = WebsocketRequestData {
        user_agent: user_agent.into(),
        user_id: UserId(NonZeroU64::new(user_id).unwrap()),
        session_id: headers
            .get("Session-Id")
            .map(|data| data.to_str().unwrap().parse::<u128>().unwrap()),
    };

    // now stop complaining compiler
    let on_error_request = request.clone();
    let on_upgrade_request = request.clone();

    websocket_upgrade
        .on_failed_upgrade(move |error| {
            handle_websocket_upgrade_error(&error, on_error_request, connection)
        })
        .on_upgrade(move |socket| {
            handle_websocket_upgrade_request(socket, on_upgrade_request, connection)
        })
}
