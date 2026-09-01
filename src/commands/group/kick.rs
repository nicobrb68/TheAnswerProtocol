use std::collections::HashMap;
use crate::{TapError, World};
use tokio::sync::Mutex;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use crate::events::group::notify_group;
use crate::events::user::notify_user;

pub async fn kick_group(
    username: &str,
    group_id: &str,
    target: &str,
    world: &Arc<Mutex<World>>,
    registry: &Arc<Mutex<HashMap<String, UnboundedSender<String>>>>
) -> String {
    if target == username {
        return TapError::CannotKickSelf.message();
    }

    let mut w = world.lock().await;

    let group = match w.get_mut_group(group_id) {
        Some(g) => g,
        None => return TapError::GroupNotFound.message(),
    };

    if group.leader != username {
        return TapError::NotGroupLeader.message();
    }

    if !group.players.contains(&target.to_string()) {
        return TapError::PlayerNotInGroup.message();
    }
    group.players.retain(|p| p != target);

    if let Some(p) = w.get_mut_player(target) {
        p.group_id = None;
    }
    drop(w);

    notify_user(target, &format!("EVT GROUP KICK {}\n", username), registry).await;
    notify_group(group_id, &format!("EVT GROUP LEAVE {}\n", target), None, world, registry).await;

    "OK\n".to_string()
}