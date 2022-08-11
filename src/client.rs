use anyhow::Result;
use std::{
    io::{BufWriter, Write},
    os::unix::net::UnixStream,
};
use tracing::log;

#[tracing::instrument]
pub fn run_client(path: &str) -> Result<()> {
    log::info!("🚀 - Starting echo client");
    let stream = UnixStream::connect(path)?;
    let mut writer = BufWriter::new(stream);
    let stdin = std::io::stdin();
    let mut line = String::new();
    loop {
        stdin.read_line(&mut line)?;
        if line.len() == 0 {
            break;
        }
        writer.write(line.as_bytes())?;
        line.clear();
    }
    Ok(())
}
