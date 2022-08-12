use anyhow::Result;
use core::time;
use ctrlc;
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::{
    fs,
    io::{BufRead, BufReader},
    os::unix::net::UnixListener,
};
use tracing::log;

#[tracing::instrument]
pub fn run_echo_server(path: &str) -> Result<()> {
    log::info!("🚀 - Starting up an echo server");

    // Create a thread-safe boolean to store "we need to shutdown right now" state
    let running = Arc::new(AtomicBool::new(true));
    {
        let running = running.clone();
        ctrlc::set_handler(move || {
            running.store(false, Ordering::SeqCst);
        })?;
    }

    // Delete if socket already open
    let _ = fs::remove_file(path);

    // Bind and listen
    let stream = UnixListener::bind(path)?;

    // Set non-blocking
    stream.set_nonblocking(true)?;

    let mut threads: Vec<JoinHandle<()>> = vec![];
    // Accept
    for stream in stream.incoming() {
        if !running.load(Ordering::SeqCst) {
            log::info!("💤 - Starting Safe Server Down");
            for thread in threads {
                thread.join().unwrap()
            }
            return Ok(());
        }

        if let Some(stream) = stream.ok() {
            let running = running.clone();
            threads.push(thread::spawn(|| handle_client(stream, running)));
        } else {
            thread::sleep(time::Duration::from_millis(50));
        }
    }
    Ok(())
}

fn handle_client(stream: UnixStream, running: Arc<AtomicBool>) {
    log::info!("⚡️ - New Connection");
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        if !running.load(Ordering::SeqCst) {
            log::info!("💤 - Shutting Down Thread Safely");
            return;
        }

        match reader.fill_buf().map(|b| !b.is_empty()) {
            Ok(has_data) => {
                if has_data {
                    // Read
                    match reader.read_line(&mut line) {
                        Ok(count) => {
                            if count == 0 {
                                // Close
                                log::info!("🚪 - Connection Closed");
                                break;
                            }
                            print!("{line}");
                            line.clear();
                        }
                        Err(_) => {}
                    };
                } else {
                    thread::sleep(time::Duration::from_millis(50));
                }
            }
            Err(_) => {
                thread::sleep(time::Duration::from_millis(50));
            }
        };
    }
}
