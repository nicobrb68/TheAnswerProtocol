use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use crate::{World, PlayerState, TapError};
use crate::events::room::notify_room;

pub async fn handle_attack(
	username: &str,
	npc_id: &str,
	world: &Arc<Mutex<World>>,
	registry: &Arc<Mutex<HashMap<String, UnboundedSender<String>>>>
) -> String {
	let mut w = world.lock().await;

	let player = match w.get_player(username) {
		Some(p) => p,
		None => return TapError::PlayerNotFound.message(),
	};

	if matches!(player.status, PlayerState::Dead) {
		return TapError::PlayerDead.message();
	}

	let current_room = player.current_room.clone();
	let player_inventory = player.inventory.clone();

	let npc_full_id = match w.get_room(&current_room)
		.and_then(|r| r.npcs.iter().find(|n| n.contains(npc_id)).cloned()) {
		Some(id) => id,
		None => return TapError::NpcNotFound.message(),
	};

	let npc = match w.get_npc(&npc_full_id) {
		Some(n) => n,
		None => return TapError::NpcNotFound.message(),
	};

	if !npc.hostile {
		return TapError::NpcNotHostile.message();
	}

	let npc_damage = npc.damage.unwrap_or(5);
	let npc_name = npc.name.clone();

	let mut weapon_bonus: u32 = 0;
	for item_id in &player_inventory {
		if let Some(item) = w.items.get(item_id) {
			if let Some(dmg) = item.damage {
				if dmg > weapon_bonus {
					weapon_bonus = dmg;
				}
			}
		}
	}
	let player_damage = 10 + weapon_bonus;

	let npc = match w.get_mut_npc(&npc_full_id) {
		Some(n) => n,
		None => return TapError::NpcNotFound.message(),
	};
	npc.hp = Some(npc.hp.unwrap_or(0).saturating_sub(player_damage));
	let npc_hp = npc.hp.unwrap_or(0);

	let player_hp = match w.get_mut_player(username) {
		Some(p) => {
			p.hp = p.hp.saturating_sub(npc_damage);
			p.hp
		},
		None => return TapError::PlayerNotFound.message(),
	};

	tracing::info!(event = "combat", player = %username, npc = %npc_name, player_damage = player_damage, npc_damage = npc_damage, player_hp = player_hp, npc_hp = npc_hp, "attack performed");

	let status;
	let mut spawn_room = String::new();

	if npc_hp == 0 {
		if let Some(room) = w.get_mut_room(&current_room) {
			room.npcs.retain(|n| n != &npc_full_id);
		}
		tracing::info!(event = "combat_victory", player = %username, npc = %npc_name, room = %current_room, "npc defeated");
		status = "victory";
	} else if player_hp == 0 {
		spawn_room = w.spawn.clone();
		if let Some(old_room) = w.get_mut_room(&current_room) {
			old_room.players.retain(|p| p != username);
		}
		if let Some(new_room) = w.get_mut_room(&spawn_room) {
			new_room.players.push(username.to_string());
		}
		if let Some(p) = w.get_mut_player(username) {
			p.hp = 50;
			p.status = PlayerState::Alive;
			p.current_room = spawn_room.clone();
		}
		tracing::info!(event = "combat_death", player = %username, npc = %npc_name, respawn = %spawn_room, "player killed");
		status = "death";
	} else {
		status = "combat";
	};

	drop(w);

	notify_room(
		&current_room,
		&format!("EVT ROOM COMBAT {} attacks {} for {} damage\n", username, npc_name, player_damage),
		Some(username),
		world,
		registry
	).await;

	if status == "victory" {
		notify_room(
			&current_room,
			&format!("EVT ROOM COMBAT {} defeated by {}\n", npc_name, username),
			Some(username),
			world,
			registry
		).await;
	} else if status == "death" {
		notify_room(
			&current_room,
			&format!("EVT ROOM COMBAT {} was killed by {}\n", username, npc_name),
			Some(username),
			world,
			registry
		).await;
		notify_room(
			&current_room,
			&format!("EVT ROOM PRESENCE LEAVE {}\n", username),
			Some(username),
			world,
			registry
		).await;
		notify_room(
			&spawn_room,
			&format!("EVT ROOM PRESENCE ENTER {}\n", username),
			Some(username),
			world,
			registry
		).await;
	}

	if status == "death" {
		format!("OK {{\"attacker_hp\": 0, \"target_hp\": {}, \"damage\": {}, \"status\": \"death\", \"respawn_room\": \"{}\", \"respawn_hp\": 50}}\n",
			npc_hp, player_damage, spawn_room)
	} else {
		format!("OK {{\"attacker_hp\": {}, \"target_hp\": {}, \"damage\": {}, \"status\": \"{}\"}}\n",
			player_hp, npc_hp, player_damage, status)
	}
}
