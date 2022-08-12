use anyhow::{Error, Result};
use core::time;
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::{fs, io::BufReader, os::unix::net::UnixListener};
use tracing::log;

use crate::message::EchoCommand;
use crate::utils::create_ctrlc_waiter;

#[tracing::instrument]
pub fn run_echo_server(path: &str) -> Result<()> {
    log::info!("🚀 - Starting up an echo server");

    let running = create_ctrlc_waiter()?;

    // Delete if socket already open
    let _ = fs::remove_file(path);

    // Bind and listen
    let listener = UnixListener::bind(path)?;

    // Set listener non-blocking so we can listen for ctrl+c
    listener.set_nonblocking(true)?;

    let mut threads: Vec<JoinHandle<()>> = vec![];
    // Accept
    for stream in listener.incoming() {
        if !running.load(Ordering::SeqCst) {
            log::info!("💤 - Starting Safe Server Down");
            // BUG - If an outstanding client does not disconnect this can block
            // forever. Can't find an API to join with a timeout - https://github.com/rust-lang/rfcs/issues/1404
            for thread in threads {
                thread.join().unwrap()
            }
            return Ok(());
        }

        match stream {
            Ok(stream) => {
                let running = running.clone();
                threads.push(thread::spawn(|| handle_client(stream, running)));
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::WouldBlock => {
                    // No data on connection, sleep and wait for more data
                    thread::sleep(time::Duration::from_millis(50));
                }
                _ => {
                    return Err(Error::new(e));
                }
            },
        }
    }
    Ok(())
}

fn handle_client(stream: UnixStream, running: Arc<AtomicBool>) {
    log::info!("🧵 - New Connection Thread");
    // Now that we are on our own thread, no need to block
    stream.set_nonblocking(false).unwrap();

    let mut reader = BufReader::new(stream);
    loop {
        if !running.load(Ordering::SeqCst) {
            log::info!("💤 - Shutting Down Thread Safely");
            return;
        }

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
