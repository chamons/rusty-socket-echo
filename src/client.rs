use anyhow::Result;
use std::{
    fs,
    io::{BufRead, BufReader},
    os::unix::net::{UnixListener, UnixStream},
    sync::{Arc, Mutex},
};
use tracing::log;

use crate::message::{EchoCommand, EchoResponse};

struct Client {
    server_stream: Arc<Mutex<UnixStream>>,
    response_socket_file: tempfile::NamedTempFile,
    response_socket: UnixListener,
    id: Arc<Mutex<Option<String>>>,
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
            id: Arc::new(Mutex::new(None)),
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
            s.spawn(|| Client::handle_server_response(&mut self.response_socket, self.id.clone()));
            s.spawn(|| Client::handle_input(self.server_stream.clone(), self.id.clone()));
        });

        let id = Client::get_id(&self.id);
        self.send(EchoCommand::Goodbye(id))?;
        Ok(())
    }

    fn get_id(id: &Arc<Mutex<Option<String>>>) -> String {
        let lock = id.lock().unwrap();
        let id: &Option<&String> = &lock.as_ref();
        (*id.as_ref().unwrap()).clone()
    }

    fn setup_ctrlc_handler(&self) -> Result<()> {
        let server_stream = self.server_stream.clone();
        let id = self.id.clone();
        ctrlc::set_handler(move || Client::shutdown(&server_stream, &id))?;
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
    fn shutdown(stream: &Arc<Mutex<UnixStream>>, id: &Arc<Mutex<Option<String>>>) {
        log::info!("👋 - Sending Goodbye");
        let id = Client::get_id(&id);
        let _ = Client::send_to_stream(EchoCommand::Goodbye(id), stream);
        log::warn!("💤 - Shutting Down Client");
        // TODO - Not cleaning up temporary file
        std::process::exit(0);
    }

    fn handle_input(stream: Arc<Mutex<UnixStream>>, id: Arc<Mutex<Option<String>>>) {
        log::info!("⌨️ - Starting Input Thread");

        let stdin = std::io::stdin();
        let mut line = String::new();
        loop {
            stdin.read_line(&mut line).unwrap();
            if line.is_empty() {
                Client::shutdown(&stream, &id);
                break;
            }
            let id = Client::get_id(&id);
            Client::send_to_stream(EchoCommand::Message(line.to_owned(), id), &stream).unwrap();
            line.clear();
        }
    }

    fn handle_server_response(listener: &mut UnixListener, id: Arc<Mutex<Option<String>>>) {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let mut reader = BufReader::new(stream);
                    loop {
                        match EchoResponse::read(&mut reader).unwrap() {
                            EchoResponse::IdAssigned(assigned_id) => {
                                *id.lock().unwrap() = Some(assigned_id);
                            }
                            EchoResponse::EchoResponse(line) => {
                                print!("> {line}");
                            }
                            EchoResponse::Goodbye() => {
                                log::info!("🔌 - Response socket closed");
                                return;
                            }
                        }
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
