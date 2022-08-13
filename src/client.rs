use anyhow::Result;
use std::{
    fs,
    io::{BufRead, BufReader},
    os::unix::net::{UnixListener, UnixStream},
    sync::{Arc, Mutex},
};
use tracing::log;

use crate::message::EchoCommand;

struct Client {
    server_stream: Arc<Mutex<UnixStream>>,
    response_socket_file: tempfile::NamedTempFile,
    response_socket: UnixListener,
}

impl Client {
    pub fn connect(server_socket_path: &str) -> Result<Self> {
        log::warn!("🚀 - Starting echo client");
        let server_stream = Arc::new(Mutex::new(UnixStream::connect(server_socket_path)?));
        let (response_socket_file, response_socket) = Client::create_response_socket()?;

        Ok(Client {
            server_stream,
            response_socket_file,
            response_socket,
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

    pub fn run(&mut self) -> Result<()> {
        self.setup_ctrlc_handler()?;

        self.send(EchoCommand::Hello(self.response_socket_file.as_ref().to_str().unwrap().to_string()))?;

        std::thread::scope(|s| {
            s.spawn(|| Client::handle_server_response(&mut self.response_socket));
            s.spawn(|| Client::handle_input(self.server_stream.clone()));
        });

        self.send(EchoCommand::Goodbye)?;
        Ok(())
    }

    fn setup_ctrlc_handler(&self) -> Result<()> {
        let server_stream = self.server_stream.clone();
        ctrlc::set_handler(move || Client::shutdown(&server_stream))?;
        Ok(())
    }

    fn send(&mut self, command: EchoCommand) -> Result<()> {
        Client::send_to_stream(command, &self.server_stream)
    }

    fn send_to_stream(command: EchoCommand, stream: &Arc<Mutex<UnixStream>>) -> Result<()> {
        command.send(&mut *stream.lock().unwrap())
    }

    // As we have a mutex around the stream, this should be safe
    // As we can't mix messages on the socket
    fn shutdown(stream: &Arc<Mutex<UnixStream>>) {
        log::info!("👋 - Sending Goodbye");
        let _ = Client::send_to_stream(EchoCommand::Goodbye, stream);
        log::warn!("💤 - Shutting Down Client");
        // TODO - Not cleaning up temporary file
        std::process::exit(0);
    }

    fn handle_input(stream: Arc<Mutex<UnixStream>>) {
        log::info!("⌨️ - Starting Input Thread");

        let stdin = std::io::stdin();
        let mut line = String::new();
        loop {
            stdin.read_line(&mut line).unwrap();
            if line.is_empty() {
                Client::shutdown(&stream);
                break;
            }
            Client::send_to_stream(EchoCommand::Message(line.to_owned()), &stream).unwrap();
            line.clear();
        }
    }

    fn handle_server_response(listener: &mut UnixListener) {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let mut reader = BufReader::new(stream);
                    let mut line = String::new();
                    loop {
                        reader.read_line(&mut line).unwrap();
                        print!("> {line}");
                        line.clear();
                    }
                }
                Err(e) => {
                    log::info!("🐛 - Error in listening to server response: {e}");
                }
            }
        }
    }
}

#[tracing::instrument]
pub fn run_client(server_socket_path: &str) -> Result<()> {
    let mut client = Client::connect(server_socket_path)?;
    client.run()
}
