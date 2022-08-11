use anyhow::Result;
use clap::Arg;

use echo::{start, utils};

fn main() -> anyhow::Result<()> {
    let m = clap::command!()
        .arg(Arg::new("verbose").short('v').action(clap::ArgAction::Count))
        .get_matches();

    init_telemetry_from_verbose(m.get_one::<u8>("verbose"))?;

    // Parse arguments and start your tool here
    start();
    Ok(())
}

// Setup default telemetry based on the number of verbose '-v' flags passed
fn init_telemetry_from_verbose(verbose_count: Option<&u8>) -> Result<()> {
    let level = match verbose_count {
        None | Some(0) => "Error",
        Some(1) => "Warn",
        Some(2) => "Info",
        Some(3) => "Debug",
        _ => "Trace",
    };
    utils::init_telemetry(level)?;
    Ok(())
}
