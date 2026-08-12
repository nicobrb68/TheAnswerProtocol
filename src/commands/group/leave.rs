use std::collections::HashMap;
use crate::{TapError, World};
use tokio::sync::Mutex;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use crate::events::group::notify_group;

pub async fn leave_group(
    username: &str,
    group_id: &str,
    world: &Arc<Mutex<World>>,
    registry: &Arc<Mutex<HashMap<String, UnboundedSender<String>>>>
) -> String {
    let mut w = world.lock().await;

    let group = match w.get_mut_group(group_id) {
        Some(g) => g,
        None => return TapError::GroupNotFound.message(),
    };

    group.players.retain(|p| p != username);

    let is_empty = group.players.is_empty();
    let new_leader = if !is_empty && group.leader == username {
        Some(group.players[0].clone())
    } else {
        None
    };

    if let Some(ref leader) = new_leader {
        group.leader = leader.clone();
    }

    w.get_mut_player(username).unwrap().group_id = None;
    drop(w);

    if is_empty {
        disband_group(username, group_id, world, registry).await;
    } else {
        if let Some(leader) = new_leader {
            notify_group(group_id, &format!("EVT GROUP LEADER {}\n", leader), None, world, registry).await;
        }
        notify_group(group_id, &format!("EVT GROUP LEAVE {}\n", username), Some(username), world, registry).await;
    }

    "OK\n".to_string()
}