use anyhow::Result;
use std::collections::HashMap;
use std::io::Write;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::{fs, io::BufReader, os::unix::net::UnixListener};
use tracing::log;

use crate::message::EchoCommand;

struct Server {
    server_socket: UnixListener,
    response_sockets: Arc<Mutex<HashMap<String, UnixStream>>>,
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

    pub fn run(&mut self) -> Result<()> {
        self.setup_ctrlc_handler()?;

        for stream in self.server_socket.incoming() {
            let stream = stream?;
            // TODO - We do not clean up the WaitHandle until shutdown, and spawn one every client
            // Can we run out?
            let response_sockets = self.response_sockets.clone();
            self.connection_threads.push(thread::spawn(|| Server::handle_client(stream, response_sockets)));
        }
        Ok(())
    }

    fn setup_ctrlc_handler(&self) -> Result<()> {
        let response_sockets = self.response_sockets.clone();
        ctrlc::set_handler(move || Server::shutdown(&response_sockets))?;
        Ok(())
    }

    fn shutdown(response_sockets: &Arc<Mutex<HashMap<String, UnixStream>>>) {
        log::warn!("💤 - Starting Safe Server Down");

        log::info!("🔌 - Shutting Down Connection Sockets");
        for stream in response_sockets.lock().unwrap().values() {
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

    fn get_stream_name(stream: &UnixStream) -> String {
        let addr = stream.local_addr().unwrap();
        let path = addr.as_pathname().unwrap();
        path.to_str().unwrap().to_owned()
    }

    fn handle_client(stream: UnixStream, response_sockets: Arc<Mutex<HashMap<String, UnixStream>>>) {
        log::info!("🧵 - New Connection Thread");

        let stream_name = Server::get_stream_name(&stream);
        let mut reader = BufReader::new(stream);
        loop {
            // Read
            match EchoCommand::read(&mut reader).unwrap() {
                EchoCommand::Hello(response_path) => {
                    log::info!("👋 - Connection Started: {response_path}");
                    let response_socket = UnixStream::connect(response_path.clone()).unwrap();
                    response_sockets.lock().unwrap().insert(stream_name.clone(), response_socket);
                }
                EchoCommand::Goodbye => {
                    // Close
                    response_sockets.lock().unwrap().remove(&stream_name);
                    log::info!("🚪 - Connection Closed");
                    return;
                }
                EchoCommand::Message(msg) => {
                    log::debug!("Received: {msg}");
                    let socket = response_sockets.lock().unwrap();
                    let mut stream = socket.get(&stream_name).unwrap();
                    stream.write_all(msg.as_bytes()).unwrap();
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
