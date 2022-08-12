use anyhow::Result;
use std::os::unix::net::UnixStream;
use tracing::log;

use crate::message::EchoCommand;

#[tracing::instrument]
pub fn run_client(path: &str) -> Result<()> {
    log::info!("🚀 - Starting echo client");

    let mut stream = UnixStream::connect(path)?;
    EchoCommand::Hello.send(&mut stream)?;

    let stdin = std::io::stdin();
    let mut line = String::new();
    loop {
        stdin.read_line(&mut line)?;
        if line.is_empty() {
            log::info!("🚪 - Session Complete");
            break;
        }
        EchoCommand::Message(line.to_owned()).send(&mut stream)?;
        line.clear();
    }
    EchoCommand::Goodbye.send(&mut stream)?;

    Ok(())
}
