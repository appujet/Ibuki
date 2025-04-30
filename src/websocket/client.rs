use crate::CLIENTS;
use crate::manager::PlayerManager;
use axum::Error;
use axum::extract::ConnectInfo;
use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket};
use dashmap::mapref::one::RefMut;
use flume::{Receiver, Sender, unbounded};
use futures::{sink::SinkExt, stream::StreamExt, stream::iter};
use songbird::id::UserId;
use std::net::SocketAddr;
use std::ops::ControlFlow;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use uuid::Uuid;

#[derive(Clone)]
pub struct WebsocketRequestData {
    pub user_agent: String,
    pub user_id: UserId,
    pub session_id: Option<u128>,
}

pub struct WebsocketClient {
    pub user_id: UserId,
    pub player_manager: Arc<PlayerManager>,
    handles: Vec<JoinHandle<()>>,
    message_sender: Sender<Message>,
    message_receiver: Receiver<Message>,
}

impl WebsocketClient {
    pub fn new(user_id: UserId) -> Self {
        let player_manager = Arc::new(PlayerManager::new());
        let (message_sender, message_receiver) = unbounded::<Message>();

        Self {
            user_id,
            player_manager,
            handles: vec![],
            message_sender,
            message_receiver,
        }
    }

    pub async fn connect(
        &mut self,
        socket: WebSocket,
        session_id: u128,
        resume: bool,
    ) -> Result<(), axum::Error> {
        for handle in &self.handles {
            handle.abort();
        }

        self.handles.clear();

        let (mut sender, mut receiver) = socket.split();

        if !resume {
            let _ = self.message_receiver.drain().collect::<Vec<Message>>();
        } else {
            let mut messages = iter(self.message_receiver.drain().map(Ok::<Message, Error>));
            sender.send_all(&mut messages).await?;
        }

        let manager = self.player_manager.clone();

        let handle = tokio::spawn(async move {
            let mut closed = false;

            while let Some(Ok(message)) = receiver.next().await {
                if let Message::Close(close_frame) = message {
                    tracing::info!(
                        "Websocket connection was closed with closing frame: {:?}",
                        close_frame
                    );

                    closed = true;
                    break;
                }
            }

            if closed {
                // todo: not hard coded and configurable
                let duration = Duration::from_secs(60);

                tracing::info!(
                    "Websocket connection was closed abruptly and is possible to be resumed within {} sec(s)",
                    duration.as_secs()
                );

                sleep(duration).await;
            }

            let connections = manager.get_connection_len();
            let players = manager.get_player_len();

            manager.destroy();

            clean_up_client(session_id);

            tracing::info!(
                "Cleaned up {} connection(s) and {} player(s)",
                connections,
                players
            );
        });

        self.handles.push(handle);

        let queue = self.message_receiver.clone();

        let handle = tokio::spawn(async move {
            while let Ok(message) = queue.recv_async().await {
                sender.send(message).await.ok();
            }
        });

        self.handles.push(handle);

        Ok(())
    }

    pub async fn disconnect(&mut self) {
        let flow = self
            .send(Message::Close(Some(CloseFrame {
                code: 1000,
                reason: Utf8Bytes::from(""),
            })))
            .await;

        if flow == ControlFlow::Break(()) {
            return;
        }

        for handle in &self.handles {
            handle.abort();
        }

        self.handles.clear();
    }

    pub async fn send(&self, message: Message) -> ControlFlow<()> {
        let result = self.message_sender.send_async(message).await;

        if let Err(error) = result {
            tracing::warn!("Failed to send message due to: {}", error);
            return ControlFlow::Break(());
        }

        ControlFlow::Continue(())
    }
}

pub async fn handle_websocket_upgrade_request(
    socket: WebSocket,
    data: WebsocketRequestData,
    addr: ConnectInfo<SocketAddr>,
) {
    let session_id = data.session_id.unwrap_or(Uuid::new_v4().to_u128_le());

    let (mut client, resume): (RefMut<'_, u128, WebsocketClient>, bool) = {
        // resumed
        if let Some(client) = CLIENTS.get_mut(&session_id) {
            (client, true)
        // existing connection and not resumed
        } else if let Some(key) = CLIENTS.iter().find_map(|client| {
            if client.user_id != data.user_id {
                return None;
            }
            Some(*client.key())
        }) {
            let (_, client) = CLIENTS.remove(&key).unwrap();
            client.player_manager.destroy();

            CLIENTS.insert(session_id, client);

            (CLIENTS.get_mut(&session_id).unwrap(), false)
        // new connection
        } else {
            let client = WebsocketClient::new(data.user_id);

            CLIENTS.insert(session_id, client);

            (CLIENTS.get_mut(&session_id).unwrap(), false)
        }
    };

    if let Err(error) = client.connect(socket, session_id, resume).await {
        tracing::warn!(
            "Socket failed to connect from: {}. [SessionId: {}] [UserId: {}] [UserAgent: {}] [Error: {:?}]",
            addr.ip(),
            session_id,
            data.user_id,
            data.user_agent,
            error
        );
        return;
    }

    tracing::info!(
        "New Connection from: {}. [SessionId: {}] [UserId: {}] [UserAgent: {}] [Resume: {}]",
        addr.ip(),
        session_id,
        data.user_id,
        data.user_agent,
        resume
    );
}

pub fn handle_websocket_upgrade_error(
    error: &axum::Error,
    data: WebsocketRequestData,
    addr: ConnectInfo<SocketAddr>,
) {
    let session_id = data
        .session_id
        .map(|id| id.to_string())
        .unwrap_or("None".to_owned());

    tracing::warn!(
        "Websocket Upgrade errored from: {}. [SessionId: {}] [UserId: {}] [UserAgent: {}] [Error: {:?}]",
        addr.ip(),
        session_id,
        data.user_id,
        data.user_agent,
        error
    );
}

fn clean_up_client(session_id: u128) {
    CLIENTS.remove(&session_id);
}
