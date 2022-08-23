use std::env;

use anyhow::Result;
use clap::Parser;
use itertools::Itertools;
use tracing::log;

pub mod message;
// pub mod client;
pub mod server;
pub mod utils;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct ToolArgs {
    /// Print additional information (pass argument one to four times for increasing detail)
    #[clap(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Run tool as an echo server
    #[clap(long, action)]
    pub server: bool,

    /// Path of the socket file
    #[clap(long, default_value_t = String::from("loopback-socket"))]
    pub socket_path: String,
}

#[tracing::instrument(skip(args))]
pub async fn run_tool(args: ToolArgs) -> Result<()> {
    log::info!("Starting tool with args: '{}'", env::args().skip(1).format(" "));
    if args.server {
        server::run_echo_server(&args.socket_path).await?;
    } else {
        // client::run_client(&args.socket_path)?;
    }
    log::info!("Shutting down");
    Ok(())
}
