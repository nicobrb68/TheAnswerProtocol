use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;

pub async fn broadcast_global(
    message: &str,
    registry: &Arc<Mutex<HashMap<String, UnboundedSender<String>>>>,
) {
    let reg = registry.lock().await;
    for tx in reg.values() {
        let _ = tx.send(message.to_string());
    }
}