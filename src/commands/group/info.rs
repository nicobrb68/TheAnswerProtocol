use crate::{TapError, World};
use tokio::sync::Mutex;
use std::sync::Arc;

pub async fn info_group(
    username: &str,
    world: &Arc<Mutex<World>>
) -> String {
    let w = world.lock().await;
    let player = match w.get_player(username) {
        Some(val) => val,
        None => return TapError::PlayerNotFound.message()
    };
    let group_id = match &player.group_id {
        Some(id) => id,
        None => return TapError::NotInGroup.message()
    };
    let group = w.get_group(group_id);

    format!("OK {}\n", serde_json::to_string(&group).unwrap())
}