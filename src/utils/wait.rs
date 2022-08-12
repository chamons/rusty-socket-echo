use anyhow::Result;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub fn create_ctrlc_waiter() -> Result<Arc<AtomicBool>> {
    // Create a thread-safe boolean to store "we need to shutdown right now" state
    let running = Arc::new(AtomicBool::new(true));
    {
        let running = running.clone();
        ctrlc::set_handler(move || {
            running.store(false, Ordering::SeqCst);
        })?;
    }
    Ok(running)
}
