use std::fs;

use anyhow::Result;
use tokio::io::BufReader;
use tokio::net::{UnixListener, UnixStream};
use tokio::signal;
use tokio::sync::{broadcast, mpsc};
use tracing::log;

use crate::message::{EchoCommand, EchoResponse};
use crate::utils::Shutdown;

struct Server {
    server_socket_path: String,
}

impl Server {
    fn new(server_socket_path: &str) -> Self {
        Server {
            server_socket_path: server_socket_path.to_owned(),
        }
    }

    pub async fn startup(server_socket_path: &str) -> Result<()> {
        let server = Server::new(server_socket_path);
        let (notify_shutdown, _) = broadcast::channel::<()>(1);
        let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel::<()>(1);

        let shutdown_copy = notify_shutdown.clone();
        tokio::select! {
            _ = server.run(shutdown_copy) => {},
            _ = signal::ctrl_c() => {}
        };
        log::warn!("💤 - Starting Safe Server Down");
        drop(notify_shutdown);
        drop(shutdown_complete_tx);
        let _ = shutdown_complete_rx.recv().await;
        log::warn!("💤 - Safe Server Complete");

        Ok(())
    }

    async fn run(&self, notify_shutdown: broadcast::Sender<()>) -> Result<()> {
        log::warn!("🚀 - Starting up an echo server");
        let server_socket = self.create_server_socket()?;

        loop {
            match server_socket.accept().await {
                Ok((stream, _)) => {
                    let mut handler = Handler::new(stream);
                    let shutdown = Shutdown::new(notify_shutdown.subscribe());
                    tokio::spawn(async move {
                        if let Err(err) = handler.run(shutdown).await {
                            log::error!("🐛 - Error from handler - {err}");
                        }
                    });
                }
                Err(e) => {
                    log::error!("🐛 - Error from server socket accept - {e}");
                }
            }
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
}

impl Handler {
    pub fn new(stream: UnixStream) -> Self {
        Handler { stream }
    }

    pub async fn run(&mut self, mut shutdown: Shutdown) -> Result<()> {
        log::info!("🧵 - New Connection Task");

        tokio::select! {
            _ = self.echo() => {},
            _ = shutdown.recv() => {
                log::info!("🧵 - Connection Task Shutdown");
            }
        }
        // They may have already shut down the socket, so ignore any errors
        log::info!("👋 - Sending Goodbye");
        let _ = EchoResponse::Goodbye().send(&mut self.stream).await;

        Ok(())
    }

    async fn echo(&mut self) -> Result<()> {
        loop {
            // Read
            let (read, _) = self.stream.split();
            let mut reader = BufReader::new(read);

            match EchoCommand::read(&mut reader).await {
                Ok(EchoCommand::Hello()) => {
                    log::info!("👋 - Connection Started");
                }
                Ok(EchoCommand::Message(msg)) => {
                    EchoResponse::EchoResponse(msg).send(&mut self.stream).await.unwrap();
                }
                Ok(EchoCommand::Goodbye()) => {
                    // Close
                    // They may have already shut down the socket, so ignore any errors
                    let _ = EchoResponse::Goodbye().send(&mut self.stream);
                    log::info!("🚪 - Connection Closed");
                    return Ok(());
                }
                Err(_) => {
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
