use anyhow::Result;
use std::{io::Write, os::unix::net::UnixStream};
use tracing::log;

#[tracing::instrument]
pub fn run_client(path: &str) -> Result<()> {
    log::info!("🚀 - Starting echo client");

    let mut stream = UnixStream::connect(path)?;
    let stdin = std::io::stdin();
    let mut line = String::new();
    loop {
        stdin.read_line(&mut line)?;
        if line.is_empty() {
            log::info!("🚪 - Session Complete");
            break;
        }
        stream.write_all(line.as_bytes())?;
        line.clear();
    }
    Ok(())
}
