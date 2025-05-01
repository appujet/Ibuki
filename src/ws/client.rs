use crate::CLIENTS;
use crate::models::lavalink::{LavalinkMessage, Ready};
use crate::voice::manager::PlayerManager;
use axum::Error;
use axum::body::Bytes;
use axum::extract::ConnectInfo;
use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket};
use flume::{Receiver, Sender, unbounded};
use futures::{sink::SinkExt, stream::StreamExt, stream::iter};
use songbird::id::UserId;
use std::net::SocketAddr;
use std::ops::ControlFlow;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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
    pub session_id: u128,
    pub player_manager: Arc<PlayerManager>,
    message_sender: Sender<Message>,
    message_receiver: Receiver<Message>,
    handles: Vec<JoinHandle<()>>,
}

impl WebsocketClient {
    pub fn new(user_id: UserId) -> Self {
        let session_id = Uuid::new_v4().as_u128();
        let player_manager = Arc::new(PlayerManager::new(user_id));
        let (message_sender, message_receiver) = unbounded::<Message>();

        Self {
            user_id,
            session_id,
            player_manager,
            message_sender,
            message_receiver,
            handles: vec![],
        }
    }

    pub async fn connect(
        &mut self,
        socket: WebSocket,
        session_id: Option<u128>,
    ) -> Result<bool, Error> {
        self.handles.retain(|handle| {
            handle.abort();
            false
        });

        let (mut sender, mut receiver) = socket.split();

        // check if the socket is open to send messages
        sender.send(Message::Ping(Bytes::new())).await?;

        let mut resumed = false;

        if session_id.filter(|id| *id == self.session_id).is_some() {
            let mut messages = iter(self.message_receiver.drain().map(Ok::<Message, Error>));

            sender.send_all(&mut messages).await?;

            resumed = true;
        } else {
            let _ = self.message_receiver.drain().collect::<Vec<Message>>();

            self.player_manager.destroy();

            self.session_id = Uuid::new_v4().as_u128();
        }

        let ptr = Arc::new(AtomicBool::new(false));

        // incoming message handler
        let dropped = ptr.clone();
        let manager = self.player_manager.clone();
        let message_sender = self.message_sender.clone();
        let user_id = self.user_id.to_owned();
        let receive_handle = tokio::spawn(async move {
            while let Some(Ok(message)) = receiver.next().await {
                if let Message::Close(close_frame) = message {
                    tracing::info!(
                        "Websocket connection was closed with closing frame: {:?}",
                        close_frame
                    );
                    break;
                }
                if let Message::Text(data) = message {
                    tracing::info!("Websocket connection received a message: {}", data.as_str());
                }
            }

            dropped.swap(true, Ordering::Acquire);

            message_sender.send_async(Message::Close(None)).await.ok();

            drop(receiver);

            // todo: not hard coded and configurable
            let duration = Duration::from_secs(15);

            tracing::info!(
                "Websocket connection was closed abruptly and is possible to be resumed within {} sec(s)",
                duration.as_secs()
            );

            sleep(duration).await;

            let connections = manager.get_connection_len();
            let players = manager.get_player_len();

            manager.destroy();

            CLIENTS.remove(&user_id);

            tracing::info!(
                "Cleaned up {} connection(s) and {} player(s)",
                connections,
                players
            );
        });

        self.handles.push(receive_handle);

        // message sender handler
        let queue = self.message_receiver.clone();
        let dropped = ptr.clone();
        let send_handle = tokio::spawn(async move {
            while let Ok(message) = queue.recv_async().await {
                if dropped.load(Ordering::Acquire) {
                    break;
                }

                sender.send(message).await.ok();
            }

            tracing::info!("Websocket connection sender is stopped");
        });

        self.handles.push(send_handle);

        let event = Ready {
            resumed,
            session_id: self.session_id.to_string(),
        };

        let serialized = serde_json::to_string(&LavalinkMessage::Ready(event)).unwrap();

        self.send(Message::Text(Utf8Bytes::from(serialized))).await;

        Ok(resumed)
    }

    /**
     * Disconnects the ws only
     */
    pub async fn disconnect(&mut self) {
        let flow = self
            .send(Message::Close(Some(CloseFrame {
                code: 1000,
                reason: Utf8Bytes::from("Invoked Disconnect"),
            })))
            .await;

        if flow == ControlFlow::Break(()) {
            return;
        }

        self.handles.retain(|handle| {
            handle.abort();
            false
        });
    }

    pub async fn send(&self, message: Message) -> ControlFlow<()> {
        let result = self.message_sender.send_async(message).await;

        if let Err(error) = result {
            tracing::warn!("Failed to send message due to: {}", error);
            return ControlFlow::Break(());
        }

        ControlFlow::Continue(())
    }

    /**
     * Disconnects without close code and clears the voice connections
     */
    pub fn destroy(&mut self) {
        self.handles.retain(|handle| {
            handle.abort();
            false
        });

        self.player_manager.destroy();
    }
}

pub async fn handle_websocket_upgrade_request(
    socket: WebSocket,
    data: WebsocketRequestData,
    addr: ConnectInfo<SocketAddr>,
) {
    let mut client = CLIENTS.get_mut(&data.user_id).unwrap_or_else(|| {
        let client = WebsocketClient::new(data.user_id);

        CLIENTS.insert(data.user_id, client);

        CLIENTS.get_mut(&data.user_id).unwrap()
    });

    match client.connect(socket, data.session_id).await {
        Ok(resumed) => {
            tracing::info!(
                "New Connection from: {}. [SessionId: {}] [UserId: {}] [UserAgent: {}] [Resume: {}]",
                addr.ip(),
                client.session_id,
                data.user_id,
                data.user_agent,
                resumed
            );
        }
        Err(error) => {
            tracing::warn!(
                "Socket failed to connect from: {}. [SessionId: {}] [UserId: {}] [UserAgent: {}] [Error: {:?}]",
                addr.ip(),
                client.session_id,
                data.user_id,
                data.user_agent,
                error
            );
        }
    };
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
