use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use crate::{World, events::room::notify_room};

pub async fn handle_disconnect(
    username: &Option<String>,
    world: &Arc<Mutex<World>>,
    registry: &Arc<Mutex<HashMap<String, UnboundedSender<String>>>>
) -> Result<(), String> {
    match username {
        Some(name) => {
            let player_room_id = {
                let w = world.lock().await;
                match w.get_player(name) {
                    Some(player) => player.current_room.clone(),
                    None => return Err("Failed to fetch user".to_string()),
                }
            };
            notify_room(&player_room_id, &format!("EVT ROOM PRESENCE LEAVE {}\n", name), Some(name), world, registry).await;
            let mut w = world.lock().await;
            let mut reg = registry.lock().await;
            reg.remove(name);
            if let Some(group_id) = w.get_player(name).and_then(|p| p.group_id.clone()) {
                if let Some(group) = w.get_mut_group(&group_id) {
                    group.players.retain(|p| p != name);
                    if group.players.is_empty() {
                        let _ = group;
                        w.remove_group(&group_id);
                    } else if group.leader == *name {
                        group.leader = group.players[0].clone();
                        let new_leader = group.leader.clone();
                        let evt_leader = format!("EVT GROUP LEADER {}\n", new_leader);
                        let evt_leave = format!("EVT GROUP LEAVE {}\n", name);
                        for member in &group.players {
                            if let Some(tx) = reg.get(member) {
                                let _ = tx.send(evt_leader.clone());
                                let _ = tx.send(evt_leave.clone());
                            }
                        }
                    } else {
                        let evt = format!("EVT GROUP LEAVE {}\n", name);
                        for member in &group.players {
                            if let Some(tx) = reg.get(member) {
                                let _ = tx.send(evt.clone());
                            }
                        }
                    }
                }
            }
            w.market.retain(|listing| listing.seller != *name);
            match w.get_mut_room(&player_room_id) {
                Some(room) => room.players.retain(|p| p != name),
                None => return Err("Failed to fetch user's room".to_string()),
            };
            w.players.remove(name);
            let count = w.players.len();
            let evt = format!("EVT STATS players={}\n", count);
            for tx in reg.values() {
                let _ = tx.send(evt.clone());
            }
            tracing::info!(event = "player_disconnect", player = %name, room = %player_room_id, "player disconnected");
        },
        None => {}
    }
    Ok(())
}
