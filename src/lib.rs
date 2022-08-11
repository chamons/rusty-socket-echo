use std::env;

use clap::ArgMatches;
use clap::{Arg, Parser};
use itertools::Itertools;
use tracing::log;

pub mod server;
pub mod utils;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct ToolArgs {
    /// Print additional information (pass argument one to four times for increasing detail)
    #[clap(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

#[tracing::instrument(skip(args))]
pub fn run_tool(args: ToolArgs) {
    log::info!("Starting tool with args: '{}'", env::args().skip(1).format(" "));
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_start() {
        assert_eq!(1, 1);
    }
}
