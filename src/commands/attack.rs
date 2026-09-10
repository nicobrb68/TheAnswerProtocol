use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use crate::{World, PlayerState, TapError};
use crate::events::room::notify_room;

const NPC_RESPAWN_SECS: u64 = 30;

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
    let npc_gold_drop = npc.gold_drop;

    let mut weapon_bonus: u32 = 0;
    let mut armor_bonus: u32 = 0;
    for item_id in &player_inventory {
        if let Some(item) = w.items.get(item_id) {
            if let Some(dmg) = item.damage {
                if dmg > weapon_bonus {
                    weapon_bonus = dmg;
                }
            }
            if let Some(arm) = item.armor {
                if arm > armor_bonus {
                    armor_bonus = arm;
                }
            }
        }
    }
    let riposte = w.get_player(username).map(|p| p.braced_bonus).unwrap_or(0);
    let player_damage = 10 + weapon_bonus + riposte;
    let absorbed = armor_bonus.min(npc_damage.saturating_sub(1));
    let effective_npc_damage = npc_damage - absorbed;

    let npc = match w.get_mut_npc(&npc_full_id) {
        Some(n) => n,
        None => return TapError::NpcNotFound.message(),
    };
    npc.hp = Some(npc.hp.unwrap_or(0).saturating_sub(player_damage));
    npc.last_hit = Some(Instant::now());
    if !npc.attackers.contains(&username.to_string()) {
        npc.attackers.push(username.to_string());
    }
    let npc_hp = npc.hp.unwrap_or(0);

    let (player_hp, effective_npc_damage) = match w.get_mut_player(username) {
        Some(p) => {
            p.braced_bonus = 0;
            p.in_combat_with = Some(npc_full_id.clone());

            if npc_hp == 0 {
                (p.hp, 0)
            } else {
                let incoming = if p.defending {
                    p.defending = false;
                    (effective_npc_damage / 2).max(1)
                } else {
                    effective_npc_damage
                };
                p.hp = p.hp.saturating_sub(incoming);
                (p.hp, incoming)
            }
        },
        None => return TapError::PlayerNotFound.message(),
    };

    tracing::info!(event = "combat", player = %username, npc = %npc_name, player_damage = player_damage, npc_damage = npc_damage, player_hp = player_hp, npc_hp = npc_hp, "attack performed");

    let status;
    let mut spawn_room = String::new();
    let mut is_boss_kill = false;
    let mut reward_notices: Vec<(String, u32)> = Vec::new();

    if npc_hp == 0 {
        if let Some(room) = w.get_mut_room(&current_room) {
            if let Some(pos) = room.npcs.iter().position(|n| n == &npc_full_id) {
                room.npcs.remove(pos);
            }
        }
        let attackers = w.get_npc(&npc_full_id)
            .map(|n| n.attackers.clone())
            .unwrap_or_default();

        if let Some(npc) = w.get_mut_npc(&npc_full_id) {
            npc.hp = npc.max_hp;
            npc.last_hit = None;
            npc.attackers.clear();
        }

        if !crate::events::boss::is_boss(&npc_full_id, &w.npcs) {
            let world_clone = Arc::clone(world);
            let registry_clone = Arc::clone(registry);
            let room_clone = current_room.clone();
            let npc_clone = npc_full_id.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(NPC_RESPAWN_SECS)).await;
                let respawned = {
                    let mut w = world_clone.lock().await;
                    match w.get_mut_room(&room_clone) {
                        Some(room) if !room.npcs.contains(&npc_clone) => {
                            room.npcs.push(npc_clone.clone());
                            tracing::info!(event = "npc_respawn", npc = %npc_clone, room = %room_clone, "npc respawned");
                            true
                        }
                        _ => false,
                    }
                };
                if respawned {
                    notify_room(
                        &room_clone,
                        &format!("EVT ROOM NPC RESPAWN {}\n", npc_clone),
                        None,
                        &world_clone,
                        &registry_clone,
                    ).await;
                }
            });
        }

        for attacker in &attackers {
            if let Some(p) = w.get_mut_player(attacker) {
                if p.in_combat_with.as_deref() == Some(npc_full_id.as_str()) {
                    p.in_combat_with = None;
                    p.defending = false;
                    p.braced_bonus = 0;
                }
                *p.kills.entry(npc_full_id.clone()).or_insert(0) += 1;
                p.gold += npc_gold_drop;
                if attacker != username {
                    reward_notices.push((attacker.clone(), p.gold));
                }
            }
        }

        tracing::info!(event = "combat_victory", player = %username, npc = %npc_name, room = %current_room, gold = npc_gold_drop, "npc defeated");
        status = "victory";
        is_boss_kill = crate::events::boss::is_boss(&npc_full_id, &w.npcs);
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
            p.in_combat_with = None;
            p.defending = false;
        }
        tracing::info!(event = "combat_death", player = %username, npc = %npc_name, respawn = %spawn_room, "player killed");
        status = "death";
    } else {
        status = "combat";
    };

    drop(w);

    notify_room(
        &current_room,
        &format!("EVT ROOM COMBAT {} attacks {} for {} damage npc={} hp={}\n", username, npc_name, player_damage, npc_full_id, npc_hp),
        Some(username),
        world,
        registry
    ).await;

    if status == "victory" {
        notify_room(
            &current_room,
            &format!("EVT ROOM COMBAT {} defeated by {} npc={}\n", npc_name, username, npc_full_id),
            Some(username),
            world,
            registry
        ).await;

        if !reward_notices.is_empty() {
            let reg = registry.lock().await;
            for (attacker, total) in &reward_notices {
                if let Some(tx) = reg.get(attacker) {
                    let _ = tx.send(format!(
                        "EVT COMBAT REWARD npc={} gold={} total={}\n",
                        npc_full_id, npc_gold_drop, total
                    ));
                }
            }
        }
        if is_boss_kill {
            let msg = format!("EVT GLOBAL [DEFEAT] {} has been slain by {}! boss={}\n", npc_name, username, npc_full_id);
            let reg = registry.lock().await;
            for tx in reg.values() {
                let _ = tx.send(msg.clone());
            }
        }
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
        format!("OK {{\"attacker_hp\": 0, \"target_hp\": {}, \"damage\": {}, \"absorbed\": {}, \"npc_damage\": {}, \"riposte\": {}, \"status\": \"death\", \"respawn_room\": \"{}\", \"respawn_hp\": 50}}\n",
            npc_hp, player_damage, absorbed, effective_npc_damage, riposte, spawn_room)
    } else if status == "victory" {
        format!("OK {{\"attacker_hp\": {}, \"target_hp\": 0, \"damage\": {}, \"absorbed\": {}, \"npc_damage\": {}, \"riposte\": {}, \"status\": \"victory\", \"gold_earned\": {}}}\n",
            player_hp, player_damage, absorbed, effective_npc_damage, riposte, npc_gold_drop)
    } else {
        format!("OK {{\"attacker_hp\": {}, \"target_hp\": {}, \"damage\": {}, \"absorbed\": {}, \"npc_damage\": {}, \"riposte\": {}, \"status\": \"combat\"}}\n",
            player_hp, npc_hp, player_damage, absorbed, effective_npc_damage, riposte)
    }
}