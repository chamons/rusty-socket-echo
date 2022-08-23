use std::fs;

use anyhow::Result;
use tokio::io::BufReader;
use tokio::net::{UnixListener, UnixStream};
use tokio::signal;
use tokio::sync::{broadcast, mpsc};
use tracing::log;
use uuid::Uuid;

use crate::message::{EchoCommand, EchoResponse};
use crate::utils::Shutdown;

struct Server {
    server_socket_path: String,
    notify_shutdown: broadcast::Sender<()>,
    shutdown_complete_rx: mpsc::Receiver<()>,
    shutdown_complete_tx: mpsc::Sender<()>,
}

impl Server {
    fn new(server_socket_path: &str) -> Self {
        let (notify_shutdown, _) = broadcast::channel::<()>(1);
        let (shutdown_complete_tx, shutdown_complete_rx) = mpsc::channel::<()>(1);

        Server {
            server_socket_path: server_socket_path.to_owned(),
            notify_shutdown,
            shutdown_complete_rx,
            shutdown_complete_tx,
        }
    }

    async fn shutdown(mut self) {
        drop(self.notify_shutdown);
        drop(self.shutdown_complete_tx);
        let _ = self.shutdown_complete_rx.recv().await;
    }

    pub async fn startup(server_socket_path: &str) -> Result<()> {
        let server = Server::new(server_socket_path);

        tokio::select! {
            res = server.run() => {
                if let Err(err) = res {
                    log::error!("🐛 - Error from server - {err}");
                }
            }
            _ = signal::ctrl_c() => {
                log::warn!("💤 - Starting Safe Server Down");
            }
        }

        server.shutdown().await;

        Ok(())
    }

    async fn run(&self) -> Result<()> {
        log::warn!("🚀 - Starting up an echo server");
        let server_socket = self.create_server_socket()?;

        loop {
            let (stream, _) = server_socket.accept().await.unwrap();
            let mut handler = Handler::new(stream, &self.notify_shutdown, &self.shutdown_complete_tx);
            tokio::spawn(async move {
                if let Err(err) = handler.run().await {
                    log::error!("🐛 - Error from handler - {err}");
                }
            });
        }
    }

    fn create_server_socket(&self) -> Result<UnixListener> {
        log::info!("🔌 - Creating server socket: {}", self.server_socket_path);

        // Delete if socket already open
        let _ = fs::remove_file(&self.server_socket_path);

        // Bind and listen
        Ok(UnixListener::bind(&self.server_socket_path)?)
    }
}

struct Handler {
    stream: UnixStream,
    response_socket: Option<UnixStream>,

    shutdown: Shutdown,
    _shutdown_complete: mpsc::Sender<()>,
}

impl Handler {
    pub fn new(stream: UnixStream, notify_shutdown: &broadcast::Sender<()>, shutdown_complete_tx: &mpsc::Sender<()>) -> Self {
        Handler {
            stream,
            response_socket: None,
            shutdown: Shutdown::new(notify_shutdown.subscribe()),
            _shutdown_complete: shutdown_complete_tx.clone(),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        log::info!("🧵 - New Connection Thread");

        loop {
            // Read
            let (read, _) = self.stream.split();
            let mut reader = BufReader::new(read);

            match EchoCommand::read(&mut reader).await.unwrap() {
                EchoCommand::Hello(response_path) => {
                    log::info!("👋 - Connection Started");
                    let mut response_socket = UnixStream::connect(response_path.clone()).await.unwrap();
                    let id = Uuid::new_v4().to_string();
                    log::info!("📖 - ID Assigned {id}");
                    EchoResponse::IdAssigned(id.clone()).send(&mut response_socket).await.unwrap();
                    self.response_socket = Some(response_socket);
                }
                EchoCommand::Message(msg, id) => {
                    log::debug!("Received: {} from {id}", msg.trim_end());
                    EchoResponse::EchoResponse(msg).send(self.response_socket.as_mut().unwrap()).await.unwrap();
                }
                EchoCommand::Goodbye(id) => {
                    // Close
                    // They may have already shut down the socket, so ignore any errors
                    let _ = EchoResponse::Goodbye().send(self.response_socket.as_mut().unwrap());
                    self.response_socket = None;
                    log::info!("🚪 - Connection Closed");
                    return Ok(());
                }
            }
        }
    }

    //             let _ = EchoResponse::Goodbye().send(stream);
}

#[tracing::instrument]
pub async fn run_echo_server(server_socket_path: &str) -> Result<()> {
    Server::startup(server_socket_path).await
}
