use crate::CLIENTS;
use crate::manager::PlayerManager;
use axum::extract::ConnectInfo;
use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket};
use flume::{Receiver, Sender, unbounded};
use futures::{sink::SinkExt, stream::StreamExt};
use songbird::id::UserId;
use std::net::SocketAddr;
use std::ops::ControlFlow;
use std::sync::Arc;
use tokio::task::JoinHandle;
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

    pub async fn connect(&mut self, socket: WebSocket) -> Result<(), axum::Error> {
        for handle in &self.handles {
            handle.abort();
        }

        self.handles.clear();

        let (mut sender, mut receiver) = socket.split();

        for message in self.message_receiver.drain() {
            sender.send(message).await?;
        }

        let handle = tokio::spawn(async move {
            while let Some(_message) = receiver.next().await {
                // todo: handle websocket message
            }

            // todo: on disconnect
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

    let flow = {
        let mut client = CLIENTS.get_mut(&session_id).unwrap_or_else(|| {
            let client = WebsocketClient::new(data.user_id);
            CLIENTS.insert(session_id, client);
            CLIENTS.get_mut(&session_id).unwrap()
        });

        if let Err(error) = client.connect(socket).await {
            tracing::warn!(
                "Socket failed to connect from: {}. [SessionId: {}] [UserId: {}] [UserAgent: {}] [Error: {:?}]",
                addr.ip(),
                session_id,
                data.user_id,
                data.user_agent,
                error
            );
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    };

    if flow == ControlFlow::Break(()) {
        CLIENTS.remove(&session_id);
        return;
    }

    tracing::info!(
        "New Connection from: {}. [SessionId: {}] [UserId: {}] [UserAgent: {}]",
        addr.ip(),
        session_id,
        data.user_id,
        data.user_agent,
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
