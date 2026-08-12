use std::collections::HashMap;
use crate::{TapError, World};
use tokio::sync::Mutex;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use crate::events::group::notify_group;

pub async fn join_group(
    username: &str,
    group_id: &str,
    world: &Arc<Mutex<World>>,
    registry: &Arc<Mutex<HashMap<String, UnboundedSender<String>>>>
) -> String {
    let mut w = world.lock().await;

    let player = match w.get_player(username) {
        Some(p) => p,
        None => return TapError::PlayerNotFound.message(),
    };
    if player.group_id.is_some() {
        return TapError::AlreadyInGroup.message();
    }

    let group = match w.get_mut_group(group_id) {
        Some(g) => g,
        None => return TapError::GroupNotFound.message(),
    };
    if !group.invited.contains(&username.to_string()) {
        return TapError::NotInvited.message();
    }

    group.invited.retain(|p| p != username);
    group.players.push(username.to_string());
    drop(w);

    let mut w = world.lock().await;
    w.get_mut_player(username).unwrap().group_id = Some(group_id.to_string());
    drop(w);

    notify_group(group_id, &format!("EVT GROUP JOIN {}\n", username), None, world, registry).await;
    format!("OK group={}\n", group_id)
}