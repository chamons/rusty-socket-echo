use tracing::log;

#[tracing::instrument]
pub fn run_echo_server(path: &str) {
    log::info!("🚀 - Starting up an echo server on socket at {}", path);
}
