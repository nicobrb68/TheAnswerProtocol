use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use crate::World;

pub async fn notify_user(
    target: &str,
    message: &str,
    registry: &Arc<Mutex<HashMap<String, UnboundedSender<String>>>>
) {
    let reg = registry.lock().await;
    if let Some(tx) = reg.get(target) {
        let _ = tx.send(message.to_string());
    }
}