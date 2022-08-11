use tracing::log;

#[tracing::instrument]
pub fn run_client(path: &str) {
    log::info!("🚀 - Starting echo client");
}
