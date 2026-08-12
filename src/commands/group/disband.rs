use std::collections::HashMap;
use crate::{TapError, World};
use tokio::sync::Mutex;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use crate::events::group::notify_group;

pub async fn disband_group(
    username: &str,
    group_id: &str,
    world: &Arc<Mutex<World>>,
    registry: &Arc<Mutex<HashMap<String, UnboundedSender<String>>>>
) -> String {
    let mut w = world.lock().await;

    let group = match w.get_group(group_id) {
        Some(g) => g,
        None => return TapError::GroupNotFound.message(),
    };

    if group.leader != username {
        return TapError::NotGroupLeader.message();
    }

    let members = group.players.clone();
    drop(w);

    notify_group(group_id, &"EVT GROUP DISBAND\n".to_string(), None, world, registry).await;

    let mut w = world.lock().await;
    for member in &members {
        if let Some(p) = w.get_mut_player(member) {
            p.group_id = None;
        }
    }
    w.groups.remove(group_id);
    drop(w);


    "OK\n".to_string()
}