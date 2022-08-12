use anyhow::Result;
use std::collections::HashMap;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::{fs, io::BufReader, os::unix::net::UnixListener};
use tracing::log;

use crate::message::EchoCommand;

struct Server {
    server_socket: UnixListener,
    response_sockets: Arc<Mutex<HashMap<UnixStream, UnixStream>>>,
    connection_threads: Vec<JoinHandle<()>>,
}

impl Server {
    pub fn startup(server_socket_path: &str) -> Result<Self> {
        log::info!("🚀 - Starting up an echo server");
        let server_socket = Server::create_server_socket(server_socket_path)?;
        Ok(Server {
            server_socket,
            response_sockets: Arc::new(Mutex::new(HashMap::new())),
            connection_threads: vec![],
        })
    }

    pub fn run(&mut self) -> Result<()> {
        self.setup_ctrlc_handler()?;

        for stream in self.server_socket.incoming() {
            let stream = stream?;
            self.connection_threads.push(thread::spawn(|| Server::handle_client(stream)));
        }
        Ok(())
    }

    fn setup_ctrlc_handler(&self) -> Result<()> {
        let response_sockets = self.response_sockets.clone();
        ctrlc::set_handler(move || Server::shutdown(&response_sockets))?;
        Ok(())
    }

    fn shutdown(response_sockets: &Arc<Mutex<HashMap<UnixStream, UnixStream>>>) {
        log::info!("💤 - Starting Safe Server Down");

        log::info!("🔌 - Shutting Down Connection Sockets");
        for stream in response_sockets.lock().unwrap().keys() {
            let _ = stream.shutdown(Shutdown::Both);
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

    fn handle_client(stream: UnixStream) {
        log::info!("🧵 - New Connection Thread");

        let mut reader = BufReader::new(stream);
        loop {
            // Read
            match EchoCommand::read(&mut reader).unwrap() {
                EchoCommand::Hello(response_path) => {
                    log::info!("👋 - Connection Started: {response_path}");
                }
                EchoCommand::Goodbye => {
                    // Close
                    log::info!("🚪 - Connection Closed");
                    return;
                }
                EchoCommand::Message(msg) => {
                    print!("{msg}");
                }
            }
        }
    }
}

#[tracing::instrument]
pub fn run_echo_server(server_socket_path: &str) -> Result<()> {
    let mut server = Server::startup(server_socket_path)?;
    server.run()
}
