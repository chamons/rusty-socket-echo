use anyhow::Result;
use std::{
    fs,
    sync::{Arc, Mutex},
};
use tokio::{
    io::BufReader,
    net::{UnixListener, UnixStream},
    signal,
    sync::{broadcast, mpsc},
};
use tracing::log;

use crate::message::{EchoCommand, EchoResponse};

struct Client {
    server_stream: Arc<Mutex<UnixStream>>,
    id: Arc<Mutex<Option<String>>>,
    notify_shutdown: broadcast::Sender<()>,
    shutdown_complete_rx: mpsc::Receiver<()>,
    shutdown_complete_tx: mpsc::Sender<()>,
}

impl Client {
    pub async fn new(server_socket_path: &str) -> Result<Self> {
        log::warn!("🚀 - Starting echo client");
        let server_stream = Arc::new(Mutex::new(UnixStream::connect(server_socket_path).await?));

        let (notify_shutdown, _) = broadcast::channel::<()>(1);
        let (shutdown_complete_tx, shutdown_complete_rx) = mpsc::channel::<()>(1);

        Ok(Client {
            server_stream,
            id: Arc::new(Mutex::new(None)),
            notify_shutdown,
            shutdown_complete_rx,
            shutdown_complete_tx,
        })
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

    pub async fn startup(server_socket_path: &str) -> Result<()> {
        let mut client = Client::new(server_socket_path).await?;

        tokio::select! {
            res = client.run() => {
                if let Err(err) = res {
                    log::error!("🐛 - Error from client - {err}");
                }
            }
            _ = signal::ctrl_c() => {
                log::warn!("💤 - Starting Safe Client Down");
            }
        }

        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        let (response_socket_file, response_socket) = Client::create_response_socket()?;

        self.send(EchoCommand::Hello(response_socket_file.as_ref().to_str().unwrap().to_string()))
            .await?;
        log::warn!("👋 - Run - Hello");

        let response = tokio::spawn(async move { Client::handle_server_response(response_socket).await });
        let input = Client::handle_input(self.server_stream.clone());
        tokio::select! {
            _ = response => {},
            _ = input => {},
        };

        let id = Client::get_id(&self.id);
        self.send(EchoCommand::Goodbye(id)).await?;
        log::warn!("👋 - Run - Goodbye");

        Ok(())
    }

    fn get_id(id: &Arc<Mutex<Option<String>>>) -> String {
        "id".to_string()
    }

    async fn send(&mut self, command: EchoCommand) -> Result<()> {
        Client::send_to_stream(command, &self.server_stream).await
    }

    async fn send_to_stream(command: EchoCommand, stream: &Arc<Mutex<UnixStream>>) -> Result<()> {
        command.send(&mut *stream.lock().unwrap()).await
    }

    // As we have a mutex around the stream, this should be safe
    // As we can't mix messages on the socket
    fn shutdown(stream: &Arc<Mutex<UnixStream>>) {
        log::info!("👋 - Sending Goodbye");
        let _ = Client::send_to_stream(EchoCommand::Goodbye("".to_string()), stream);
        log::warn!("💤 - Shutting Down Client");
        // TODO - Not cleaning up temporary file
        std::process::exit(0);
    }

    async fn handle_input(stream: Arc<Mutex<UnixStream>>) {
        log::info!("⌨️ - Starting Input Task");

        let stdin = std::io::stdin();
        let mut line = String::new();
        loop {
            stdin.read_line(&mut line).unwrap();
            if line.is_empty() {
                Client::shutdown(&stream);
                break;
            }
            Client::send_to_stream(EchoCommand::Message(line.to_owned(), "".to_string()), &stream)
                .await
                .unwrap();
            line.clear();
        }
        log::info!("⌨️ - Ending Input Task");
    }

    async fn handle_server_response(listener: UnixListener) -> Result<()> {
        log::info!("⌨️ - Starting Server Response Task");
        log::info!("1️⃣ - Before Accept");
        let stream = listener.accept().await;
        log::info!("2️⃣ - After Accept");
        let (stream, _) = stream?;
        log::info!("2️⃣ - After Accept Unwrap");
        let mut reader = BufReader::new(stream);

        loop {
            match EchoResponse::read(&mut reader).await? {
                EchoResponse::IdAssigned(assigned_id) => {
                    log::info!("✏️ - Received ID: {assigned_id}");
                }
                EchoResponse::EchoResponse(line) => {
                    log::info!("✏️ - Received Response: {line}");
                    print!("> {line}");
                }
                EchoResponse::Goodbye() => {
                    log::info!("🔌 - Response socket closed");
                    log::info!("⌨️ - Ending Server Response Task");
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
