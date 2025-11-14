//! CDP WebSocket server implementation

use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::{handshake::server::Request, Message};
use tokio_tungstenite::{accept_hdr_async, WebSocketStream};
use tracing::{debug, error, info, warn};

use crate::config::ServerConfig;
use crate::error::{CdpServerError, Result};
use crate::session::{Session, SessionId, SessionState};
use crate::transport::{
    parse_cdp_message, serialize_cdp_message, validate_message_size, validate_origin,
};

/// CDP WebSocket server
pub struct CdpWebSocketServer {
    /// Server configuration
    config: ServerConfig,

    /// Active sessions
    sessions: Arc<DashMap<SessionId, Arc<parking_lot::RwLock<Session>>>>,
}

impl CdpWebSocketServer {
    /// Create a new CDP WebSocket server
    pub fn new(config: ServerConfig) -> Result<Self> {
        Ok(Self {
            config,
            sessions: Arc::new(DashMap::new()),
        })
    }

    /// Get reference to sessions map
    pub fn get_sessions(&self) -> Arc<DashMap<SessionId, Arc<parking_lot::RwLock<Session>>>> {
        Arc::clone(&self.sessions)
    }

    /// Start the WebSocket server
    pub async fn start(&self) -> Result<()> {
        let addr: SocketAddr = format!("{}:{}", self.config.bind_address, self.config.port)
            .parse()
            .map_err(|e| CdpServerError::Other(anyhow::anyhow!("Invalid address: {}", e)))?;

        let listener = TcpListener::bind(&addr).await?;
        info!("CDP WebSocket server listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    debug!("New connection from {}", peer_addr);
                    let sessions = Arc::clone(&self.sessions);
                    let config = self.config.clone();

                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, sessions, config).await {
                            error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    /// Handle a single WebSocket connection
    async fn handle_connection(
        stream: TcpStream,
        sessions: Arc<DashMap<SessionId, Arc<parking_lot::RwLock<Session>>>>,
        config: ServerConfig,
    ) -> Result<()> {
        // Accept WebSocket connection with header validation
        let allowed_origins = config.allowed_origins.clone();
        let callback = move |req: &Request, response: http::Response<()>| {
            // Validate Origin header
            if let Some(origin) = req.headers().get("Origin") {
                if let Ok(origin_str) = origin.to_str() {
                    if !validate_origin(origin_str, &allowed_origins) {
                        warn!("Rejected connection from invalid origin: {}", origin_str);
                        return Err(http::Response::builder()
                            .status(403)
                            .body(Some("Forbidden".to_string()))
                            .unwrap());
                    }
                } else {
                    warn!("Invalid Origin header");
                    return Err(http::Response::builder()
                        .status(400)
                        .body(Some("Bad Request".to_string()))
                        .unwrap());
                }
            }

            Ok(response)
        };

        let ws_stream = accept_hdr_async(stream, callback).await.map_err(Box::new)?;
        debug!("WebSocket connection established");

        // Create session
        let session_id = SessionId::new();
        let session = Arc::new(parking_lot::RwLock::new(Session::new(session_id)));
        sessions.insert(session_id, Arc::clone(&session));

        info!("Session created: {}", session_id);

        // Handle messages
        if let Err(e) = Self::handle_messages(ws_stream, Arc::clone(&session), &config).await {
            error!("Message handling error: {}", e);
        }

        // Clean up session
        session.write().close();
        sessions.remove(&session_id);
        info!("Session closed: {}", session_id);

        Ok(())
    }

    /// Handle WebSocket messages for a session
    async fn handle_messages(
        ws_stream: WebSocketStream<TcpStream>,
        session: Arc<parking_lot::RwLock<Session>>,
        config: &ServerConfig,
    ) -> Result<()> {
        let (mut write, mut read) = ws_stream.split();

        loop {
            tokio::select! {
                // Handle incoming messages
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            // Validate message size
                            if let Err(e) = validate_message_size(&text, config.max_message_size) {
                                error!("Message too large: {}", e);
                                write.send(Message::Close(None))
                                    .await
                                    .map_err(Box::new)?;
                                break;
                            }

                            // Parse CDP message
                            match parse_cdp_message(&text) {
                                Ok(cdp_msg) => {
                                    debug!("Received CDP message: {:?}", cdp_msg);

                                    // For now, echo back a simple response
                                    // In real implementation, this would be handled by protocol handler
                                    let response = Self::create_echo_response(&cdp_msg);
                                    if let Ok(response_json) = serialize_cdp_message(&response) {
                                        write.send(Message::Text(response_json))
                                    .await
                                    .map_err(Box::new)?;
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to parse CDP message: {}", e);
                                }
                            }
                        }
                        Some(Ok(Message::Ping(data))) => {
                            write.send(Message::Pong(data))
                                .await
                                .map_err(Box::new)?;
                        }
                        Some(Ok(Message::Pong(_))) => {
                            // Ignore pong messages
                        }
                        Some(Ok(Message::Close(_))) => {
                            debug!("Client closed connection");
                            break;
                        }
                        Some(Ok(Message::Binary(_))) => {
                            warn!("Binary messages not supported");
                        }
                        Some(Ok(Message::Frame(_))) => {
                            // Raw frames are handled internally
                        }
                        Some(Err(e)) => {
                            error!("WebSocket error: {}", e);
                            break;
                        }
                        None => {
                            debug!("Connection closed");
                            break;
                        }
                    }
                }
            }

            // Check if session is closed
            if session.read().state() == SessionState::Closed {
                break;
            }
        }

        Ok(())
    }

    /// Create an echo response for testing
    fn create_echo_response(msg: &cdp_types::CdpMessage) -> cdp_types::CdpMessage {
        use cdp_types::{CdpMessage, CdpResponse};
        use serde_json::json;

        match msg {
            CdpMessage::Request(req) => CdpMessage::Response(CdpResponse {
                id: req.id,
                result: Some(json!({"status": "ok"})),
                error: None,
            }),
            _ => msg.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let config = ServerConfig::default();
        let server = CdpWebSocketServer::new(config);
        assert!(server.is_ok());
    }

    #[test]
    fn test_server_sessions_empty() {
        let config = ServerConfig::default();
        let server = CdpWebSocketServer::new(config).unwrap();
        assert_eq!(server.get_sessions().len(), 0);
    }
}
