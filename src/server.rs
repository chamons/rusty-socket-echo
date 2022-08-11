use anyhow::Result;
use std::{
    fs,
    io::{BufRead, BufReader},
    os::unix::net::UnixListener,
};
use tracing::log;

#[tracing::instrument]
pub fn run_echo_server(path: &str) -> Result<()> {
    log::info!("🚀 - Starting up an echo server");

    // Delete if socket already open
    let _ = fs::remove_file(path);

    // Bind and listen
    let stream = UnixListener::bind(path)?;

    // Accept
    for stream in stream.incoming() {
        log::info!("⚡️ - New Connection");

        let stream = stream?;
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        loop {
            // Read
            let count = reader.read_line(&mut line)?;
            if count == 0 {
                log::info!("🚪 - Connection Closed");
                break;
            }
            print!("{line}");
            line.clear();
        }
        // Close
    }
    Ok(())
}
