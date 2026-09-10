use std::sync::Arc;
use std::collections::HashMap;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use crate::{World, PlayerState, TapError};
use crate::events::room::notify_room;

const FLEE_SUCCESS_PERCENT: u32 = 70;

/// No RNG crate for a single dice roll: `RandomState` is seeded by the OS and
/// re-keyed on each construction. The clock is *not* usable here — `subsec_nanos()`
/// is microsecond-granular on macOS, so `% 100` was always 0 and every escape worked.
fn roll() -> u32 {
    (RandomState::new().build_hasher().finish() % 100) as u32
}

pub async fn handle_flee(
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
    let came_from = player.entered_from.clone();
    let player_inventory = player.inventory.clone();

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

    // You retreat the way you came in; that also keeps a guarded room honest.
    let escape_room = came_from
        .filter(|r| w.get_room(&current_room).map(|c| c.exits.values().any(|t| t == r)).unwrap_or(false))
        .or_else(|| w.get_room(&current_room).and_then(|r| r.exits.values().next().cloned()));

    let escape_room = match escape_room {
        Some(r) => r,
        None => return TapError::NoExit.message(),
    };

    if roll() >= FLEE_SUCCESS_PERCENT {
        // Botched it: the NPC gets a free swing and the fight goes on.
        let mut armor_bonus: u32 = 0;
        for item_id in &player_inventory {
            if let Some(arm) = w.items.get(item_id).and_then(|i| i.armor) {
                if arm > armor_bonus { armor_bonus = arm; }
            }
        }
        let absorbed = armor_bonus.min(npc_damage.saturating_sub(1));
        let incoming = npc_damage - absorbed;

        let (player_hp, died) = match w.get_mut_player(username) {
            Some(p) => { p.hp = p.hp.saturating_sub(incoming); (p.hp, p.hp == 0) }
            None => return TapError::PlayerNotFound.message(),
        };
        tracing::warn!(event = "combat_flee_failed", player = %username, npc = %npc_name, damage_taken = incoming, player_hp = player_hp, "escape failed");

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
        }
        drop(w);

        notify_room(&current_room, &format!("EVT ROOM COMBAT {} tries to flee {} and fails npc={}\n", username, npc_name, npc_full_id), Some(username), world, registry).await;
        if died {
            notify_room(&current_room, &format!("EVT ROOM COMBAT {} was killed by {}\n", username, npc_name), Some(username), world, registry).await;
            notify_room(&current_room, &format!("EVT ROOM PRESENCE LEAVE {}\n", username), Some(username), world, registry).await;
            notify_room(&spawn_room, &format!("EVT ROOM PRESENCE ENTER {}\n", username), Some(username), world, registry).await;
            return format!("OK {{\"fled\": false, \"attacker_hp\": 50, \"npc_damage\": {}, \"status\": \"death\"}}\n", incoming);
        }
        return format!("OK {{\"fled\": false, \"attacker_hp\": {}, \"npc_damage\": {}, \"status\": \"combat\"}}\n", player_hp, incoming);
    }

    if let Some(r) = w.get_mut_room(&current_room) { r.players.retain(|p| p != username); }
    if let Some(r) = w.get_mut_room(&escape_room) { r.players.push(username.to_string()); }
    if let Some(p) = w.get_mut_player(username) {
        p.current_room = escape_room.clone();
        p.entered_from = Some(current_room.clone());
        p.in_combat_with = None;
        p.defending = false;
    }
    tracing::info!(event = "combat_flee", player = %username, npc = %npc_name, from = %current_room, to = %escape_room, "escaped combat");

    drop(w);

    notify_room(&current_room, &format!("EVT ROOM COMBAT {} flees from {} npc={}\n", username, npc_name, npc_full_id), Some(username), world, registry).await;
    notify_room(&current_room, &format!("EVT ROOM PRESENCE LEAVE {}\n", username), Some(username), world, registry).await;
    notify_room(&escape_room, &format!("EVT ROOM PRESENCE ENTER {}\n", username), Some(username), world, registry).await;

    format!("OK {{\"fled\": true, \"room\": \"{}\", \"status\": \"fled\"}}\n", escape_room)
}
