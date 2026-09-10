use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use crate::{World, PlayerState, TapError};
use crate::events::room::notify_room;

/// Give up this round's strike to brace. The NPC still takes its turn, but the
/// blow is halved on top of whatever the armor already absorbs.
pub async fn handle_defend(
    username: &str,
    world: &Arc<Mutex<World>>,
    registry: &Arc<Mutex<HashMap<String, UnboundedSender<String>>>>,
) -> String {
    let mut w = world.lock().await;

    let player = match w.get_player(username) {
        Some(p) => p,
        None => return TapError::PlayerNotFound.message(),
    };
    if matches!(player.status, PlayerState::Dead) {
        return TapError::PlayerDead.message();
    }

    let npc_full_id = match &player.in_combat_with {
        Some(id) => id.clone(),
        None => return TapError::NotInCombat.message(),
    };
    let current_room = player.current_room.clone();
    let player_inventory = player.inventory.clone();

    // Walking out of the room ends the fight, so a stale opponent means no combat.
    if !w.get_room(&current_room).map(|r| r.npcs.contains(&npc_full_id)).unwrap_or(false) {
        if let Some(p) = w.get_mut_player(username) {
            p.in_combat_with = None;
            p.defending = false;
        }
        return TapError::NotInCombat.message();
    }

    let (npc_damage, npc_name) = match w.get_npc(&npc_full_id) {
        Some(n) => (n.damage.unwrap_or(5), n.name.clone()),
        None => return TapError::NpcNotFound.message(),
    };

    let mut armor_bonus: u32 = 0;
    for item_id in &player_inventory {
        if let Some(arm) = w.items.get(item_id).and_then(|i| i.armor) {
            if arm > armor_bonus {
                armor_bonus = arm;
            }
        }
    }

    let absorbed = armor_bonus.min(npc_damage.saturating_sub(1));
    let after_armor = npc_damage - absorbed;
    let incoming = (after_armor / 2).max(1);
    let blocked = after_armor - incoming;

    let npc_hp = w.get_npc(&npc_full_id).and_then(|n| n.hp).unwrap_or(0);

    let (player_hp, died) = match w.get_mut_player(username) {
        Some(p) => {
            p.hp = p.hp.saturating_sub(incoming);
            // Braced for the *next* incoming strike too, so holding the line pays off.
            p.defending = true;
            (p.hp, p.hp == 0)
        }
        None => return TapError::PlayerNotFound.message(),
    };

    tracing::info!(event = "combat_defend", player = %username, npc = %npc_name, absorbed = absorbed, blocked = blocked, damage_taken = incoming, player_hp = player_hp, "player defended");

    let mut spawn_room = String::new();
    if died {
        spawn_room = w.spawn.clone();
        if let Some(r) = w.get_mut_room(&current_room) { r.players.retain(|p| p != username); }
        if let Some(r) = w.get_mut_room(&spawn_room) { r.players.push(username.to_string()); }
        if let Some(p) = w.get_mut_player(username) {
            p.hp = 50;
            p.status = PlayerState::Alive;
            p.current_room = spawn_room.clone();
            p.in_combat_with = None;
            p.defending = false;
        }
        tracing::info!(event = "combat_death", player = %username, npc = %npc_name, respawn = %spawn_room, "player killed while defending");
    }

    drop(w);

    notify_room(
        &current_room,
        &format!("EVT ROOM COMBAT {} braces against {} npc={}\n", username, npc_name, npc_full_id),
        Some(username),
        world,
        registry,
    ).await;

    if died {
        notify_room(&current_room, &format!("EVT ROOM COMBAT {} was killed by {}\n", username, npc_name), Some(username), world, registry).await;
        notify_room(&current_room, &format!("EVT ROOM PRESENCE LEAVE {}\n", username), Some(username), world, registry).await;
        notify_room(&spawn_room, &format!("EVT ROOM PRESENCE ENTER {}\n", username), Some(username), world, registry).await;
        return format!("OK {{\"attacker_hp\": 50, \"target_hp\": {}, \"blocked\": {}, \"npc_damage\": {}, \"status\": \"death\"}}\n",
            npc_hp, blocked, incoming);
    }

    format!("OK {{\"attacker_hp\": {}, \"target_hp\": {}, \"blocked\": {}, \"npc_damage\": {}, \"status\": \"defend\"}}\n",
        player_hp, npc_hp, blocked, incoming)
}
