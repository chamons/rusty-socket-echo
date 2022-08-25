use anyhow::Result;
use std::fs;
use tokio::{
    io::BufReader,
    net::{UnixListener, UnixStream},
    signal,
    sync::mpsc,
};
use tracing::log;

use crate::message::{EchoCommand, EchoResponse};

struct Client {}

impl Client {
    pub async fn new() -> Result<Self> {
        Ok(Client {})
    }
    pub async fn startup(server_socket_path: &str) -> Result<()> {
        let mut client = Client::new().await?;
        client.run(server_socket_path).await
    }

    pub async fn run(&mut self, server_socket_path: &str) -> Result<()> {
        log::warn!("🚀 - Starting echo client");

        let mut server_stream = UnixStream::connect(server_socket_path).await?;

        let (response_socket_file, response_socket) = Client::create_response_socket()?;
        let response_file_path = response_socket_file.as_ref().to_str().unwrap().to_string();
        EchoCommand::Hello(response_file_path).send(&mut server_stream).await?;

        tokio::select! {
            _ = Client::handle_server_response(response_socket) => {},
            _ = Client::handle_input(server_stream) => {},
            _ = signal::ctrl_c() => {}
        };
        log::warn!("👋 - Quitting echo client");

        Ok(())
    }

    fn create_response_socket() -> Result<(tempfile::NamedTempFile, UnixListener)> {
        let response_socket = tempfile::NamedTempFile::new()?;
        let response_socket_path = response_socket.as_ref();

        log::info!("Response Socket: {}", response_socket_path.display());

        let _ = fs::remove_file(&response_socket_path);
        let listener = UnixListener::bind(response_socket_path)?;
        // Bind and listen on the response socket
        Ok((response_socket, listener))
    }

    async fn handle_input(mut stream: UnixStream) {
        log::info!("Starting Input Task");

        let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel::<String>(1);

        // Spawn a thread instead of a task as it will block shutdown in ctrl+c case - https://github.com/tokio-rs/tokio/issues/2466
        std::thread::spawn(move || {
            log::info!("🧵 - Starting Input Read Thread");

            let stdin = std::io::stdin();
            loop {
                let mut line = String::new();
                stdin.read_line(&mut line).unwrap();
                if line.is_empty() {
                    break;
                }
                shutdown_complete_tx.blocking_send(line.to_owned()).unwrap();
            }
        });

        #[allow(clippy::while_let_loop)] // Loop makes task lifetime more clear
        tokio::spawn(async move {
            log::info!("Starting Input Send Task");
            loop {
                match shutdown_complete_rx.recv().await {
                    Some(line) => {
                        EchoCommand::Message(line).send(&mut stream).await.unwrap();
                    }
                    None => {
                        // Empty string sent means EOF, closing time.
                        break;
                    }
                }
            }
        })
        .await
        .unwrap();
        log::info!("⌨️ - Ending Input Task");
    }

    async fn handle_server_response(listener: UnixListener) -> Result<()> {
        log::info!("⌨️ - Starting Server Response Task");
        let stream = listener.accept().await;
        let (stream, _) = stream?;
        let mut reader = BufReader::new(stream);

        loop {
            match EchoResponse::read(&mut reader).await? {
                EchoResponse::EchoResponse(line) => {
                    print!("> {line}");
                }
                EchoResponse::Goodbye() => {
                    log::info!("🔌 - Response socket closed");
                    return Ok(());
                }
            }
        }
    }
}

#[tracing::instrument]
pub async fn run_client(server_socket_path: &str) -> Result<()> {
    Client::startup(server_socket_path).await
}
