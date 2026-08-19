use crate::{TapError, World};
use tokio::sync::Mutex;
use std::sync::Arc;

pub async fn info_group(
    group_id: &str,
    world: &Arc<Mutex<World>>
) -> String {
    let w = world.lock().await;
    let group = match w.get_group(group_id) {
        Some(g) => g,
        None => return TapError::GroupNotFound.message()
    };

    match serde_json::to_string(&group) {
        Ok(json) => format!("OK {}\n", json),
        Err(_) => TapError::SendFailed.message(),
    }
}