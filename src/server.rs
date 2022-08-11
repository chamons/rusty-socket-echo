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

    let stream = UnixListener::bind(path)?;
    for stream in stream.incoming() {
        let mut reader = BufReader::new(stream?);
        let mut line = String::new();
        loop {
            reader.read_line(&mut line)?;
            println!("{line}");
            line.clear();
        }
    }
    Ok(())
}
