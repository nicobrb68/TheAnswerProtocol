use std::collections::HashMap;
use crate::{TapError, World};
use tokio::sync::Mutex;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use crate::events::user::notify_user;

pub async fn invite_group(
    username: &str,
    group_id: &str,
    group_args: &str,
    world: &Arc<Mutex<World>>,
    registry: &Arc<Mutex<HashMap<String, UnboundedSender<String>>>>
) -> String {

    if group_args == username {
        return TapError::CannotInviteSelf.message();
    }

    let mut w = world.lock().await;

    if !w.has_player(group_args) {
        return TapError::PlayerNotFound.message();
    }

    let group = match w.get_mut_group(group_id) {
        Some(g) => g,
        None => return TapError::GroupNotFound.message(),
    };

    if group.leader != username {
        return TapError::NotGroupLeader.message();
    }

    if group.players.contains(&group_args.to_string()) || group.leader == group_args.to_string() {
        return TapError::PlayerAlreadyInGroup.message();
    };

    notify_user(&group_args, &format!("EVT GROUP INVITE {} id={}\n", username, group_id), registry).await;

    group.invited.push(group_args.to_string());

    "OK\n".to_string()
}