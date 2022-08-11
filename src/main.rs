use anyhow::Result;

use clap::Parser;
use echo::{run_tool, utils, ToolArgs};

fn main() -> anyhow::Result<()> {
    let args = ToolArgs::parse();

    init_telemetry_from_verbose(args.verbose)?;

    // Parse arguments and start your tool here
    run_tool(args);
    Ok(())
}

// Setup default telemetry based on the number of verbose '-v' flags passed
fn init_telemetry_from_verbose(verbose_count: u8) -> Result<()> {
    let level = match verbose_count {
        0 => "Error",
        1 => "Warn",
        2 => "Info",
        3 => "Debug",
        _ => "Trace",
    };
    utils::init_telemetry(level)?;
    Ok(())
}
