use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use tokio::io::AsyncWriteExt;
use tokio::io::{BufReader, BufWriter};
use tokio::net::unix::WriteHalf;
use tokio::net::{UnixListener, UnixStream};
use tokio::task::JoinHandle;
use tracing::log;
use uuid::Uuid;

use crate::message::{EchoCommand, EchoResponse};

type ResponseSockets = Arc<Mutex<HashMap<String, BufWriter<WriteHalf<'static>>>>>;

struct Server {
    server_socket: UnixListener,
    response_sockets: ResponseSockets,
    connection_threads: Vec<JoinHandle<()>>,
}

impl Server {
    pub fn startup(server_socket_path: &str) -> Result<Self> {
        log::warn!("🚀 - Starting up an echo server");
        let server_socket = Server::create_server_socket(server_socket_path)?;
        Ok(Server {
            server_socket,
            response_sockets: Arc::new(Mutex::new(HashMap::new())),
            connection_threads: vec![],
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        self.setup_ctrlc_handler()?;

        loop {
            let (stream, _) = self.server_socket.accept().await?;
            let response_sockets = self.response_sockets.clone();
            self.connection_threads
                .push(tokio::spawn(async move { Server::handle_client(stream, response_sockets) }));
        }
    }

    fn setup_ctrlc_handler(&self) -> Result<()> {
        let response_sockets = self.response_sockets.clone();
        ctrlc::set_handler(move || Server::shutdown(&response_sockets))?;
        Ok(())
    }

    fn shutdown(response_sockets: &ResponseSockets) {
        log::warn!("💤 - Starting Safe Server Down");

        log::info!("🔌 - Shutting Down Connection Sockets");
        for mut stream in response_sockets.lock().unwrap().values() {
            let _ = EchoResponse::Goodbye().send(stream);
            let _ = stream.shutdown();
        }

        std::process::exit(0);
    }

    fn create_server_socket(server_socket_path: &str) -> Result<UnixListener> {
        log::info!("🔌 - Creating server socket: {server_socket_path}");

        // Delete if socket already open
        let _ = fs::remove_file(server_socket_path);

        // Bind and listen
        Ok(UnixListener::bind(server_socket_path)?)
    }

    async fn handle_client(mut stream: UnixStream, response_sockets: ResponseSockets) {
        log::info!("🧵 - New Connection Thread");

        loop {
            // Read
            let (read, write) = stream.split();
            let mut reader = BufReader::new(read);

            match EchoCommand::read(&mut reader).await.unwrap() {
                EchoCommand::Hello(response_path) => {
                    log::info!("👋 - Connection Started");
                    let mut response_socket = UnixStream::connect(response_path.clone()).await.unwrap();
                    let id = Uuid::new_v4().to_string();
                    log::info!("📖 - ID Assigned {id}");
                    EchoResponse::IdAssigned(id.clone()).send(&mut response_socket).await.unwrap();
                    let (_, writer) = response_socket.split();
                    response_sockets.lock().unwrap().insert(id, BufWriter::new(writer));
                }
                EchoCommand::Message(msg, id) => {
                    log::debug!("Received: {} from {id}", msg.trim_end());
                    let mut response_sockets = response_sockets.lock().unwrap();
                    let writer = response_sockets.get_mut(&id).unwrap();
                    EchoResponse::EchoResponse(msg).send(writer).await.unwrap();
                }
                EchoCommand::Goodbye(id) => {
                    // Close
                    let mut response_sockets = response_sockets.lock().unwrap();
                    let writer = response_sockets.get_mut(&id).unwrap();
                    // They may have already shut down the socket, so ignore any errors
                    let _ = EchoResponse::Goodbye().send(writer);
                    response_sockets.remove(&id);
                    log::info!("🚪 - Connection Closed");
                    return;
                }
            }
        }
    }
}

#[tracing::instrument]
pub async fn run_echo_server(server_socket_path: &str) -> Result<()> {
    let mut server = Server::startup(server_socket_path)?;
    server.run().await
}
