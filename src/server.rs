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
        tokio::spawn(async move {
            signal::ctrl_c().await.unwrap();
            log::warn!("💤 - Starting Safe Server Down");
            drop(notify_shutdown);
            drop(shutdown_complete_tx);
            let _ = shutdown_complete_rx.recv().await;
            log::warn!("💤 - Safe Server Complete");
        });
        server.run(shutdown_copy).await?;
        log::warn!("💤 - Startup Complete");

        Ok(())
    }

    async fn run(&self, notify_shutdown: broadcast::Sender<()>) -> Result<()> {
        log::warn!("🚀 - Starting up an echo server");
        let server_socket = self.create_server_socket()?;

        let mut server_shutdown = Shutdown::new(notify_shutdown.subscribe());

        let process = tokio::spawn(async move {
            loop {
                let (stream, _) = server_socket.accept().await.unwrap();
                let mut handler = Handler::new(stream);
                let shutdown = Shutdown::new(notify_shutdown.subscribe());
                tokio::spawn(async move {
                    if let Err(err) = handler.run(shutdown).await {
                        log::error!("🐛 - Error from handler - {err}");
                    }
                });
            }
        });
        let shutdown = tokio::spawn(async move {
            server_shutdown.recv().await;
        });
        log::error!("Start of run");

        tokio::select! {
            _ = process => {},
            _ = shutdown => {
                log::error!("Got server_shutdown");
            }
        }
        log::error!("End of run");

        Ok(())
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
}

impl Handler {
    pub fn new(stream: UnixStream) -> Self {
        Handler { stream, response_socket: None }
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
        let _ = EchoResponse::Goodbye().send(self.response_socket.as_mut().unwrap());

        Ok(())
    }

    async fn echo(&mut self) -> Result<()> {
        loop {
            // Read
            let (read, _) = self.stream.split();
            let mut reader = BufReader::new(read);

            match EchoCommand::read(&mut reader).await {
                Ok(EchoCommand::Hello(response_path)) => {
                    log::info!("👋 - Connection Started");
                    let response_socket = UnixStream::connect(response_path.clone()).await.unwrap();
                    self.response_socket = Some(response_socket);
                }
                Ok(EchoCommand::Message(msg)) => {
                    EchoResponse::EchoResponse(msg).send(self.response_socket.as_mut().unwrap()).await.unwrap();
                }
                Ok(EchoCommand::Goodbye()) => {
                    // Close
                    // They may have already shut down the socket, so ignore any errors
                    let _ = EchoResponse::Goodbye().send(self.response_socket.as_mut().unwrap());
                    self.response_socket = None;
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
