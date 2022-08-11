use tracing::log;

pub mod utils;

#[tracing::instrument]
pub fn start() {
    println!("Hello, world!");

    log::info!("an example trace log");
    log::warn!("Warning");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_start() {
        assert_eq!(1, 1);
    }
}
