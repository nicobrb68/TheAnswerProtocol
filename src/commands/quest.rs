use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{World, TapError};

/// How far along an active quest is, as (current, required).
fn quest_progress(player: &crate::Player, quest: &crate::Quest) -> (u32, u32) {
    let current = match quest.quest_type.as_str() {
        "kill" => *player.kills.get(&quest.target_npc).unwrap_or(&0),
        // `deliver` hands the goods over on accept, so holding them is the progress.
        _ => player.inventory.iter().filter(|i| *i == &quest.target_item).count() as u32,
    };
    (current.min(quest.target_count), quest.target_count)
}

fn complete_quest(username: &str, quest_id: &str, quest: &crate::Quest, w: &mut World) -> String {
    let mut to_remove = quest.target_count;
    if let Some(p) = w.get_mut_player(username) {
        match quest.quest_type.as_str() {
            "kill" => {
                if let Some(count) = p.kills.get_mut(&quest.target_npc) {
                    *count = count.saturating_sub(quest.target_count);
                }
            }
            _ => p.inventory.retain(|i| {
                if i == &quest.target_item && to_remove > 0 {
                    to_remove -= 1;
                    false
                } else {
                    true
                }
            }),
        }
        for _ in 0..quest.reward_count {
            p.inventory.push(quest.reward.clone());
        }
        p.quests_active.retain(|q| q != quest_id);
        p.quests_done.push(quest_id.to_string());
    }
    tracing::info!(event = "quest_complete", player = %username, quest = %quest_id, kind = %quest.quest_type, reward = %quest.reward, "quest completed");

    format!("OK {{\"quest_id\": \"{}\", \"status\": \"completed\", \"reward\": \"{}\", \"reward_count\": {}}}\n",
        quest_id, quest.reward, quest.reward_count)
}

pub async fn handle_quest(username: &str, npc_id: &str, world: &Arc<Mutex<World>>) -> String {
    let mut w = world.lock().await;

    let current_room = match w.get_player(username) {
        Some(p) => p.current_room.clone(),
        None => return TapError::PlayerNotFound.message(),
    };

    let npc_full_id = match w.get_room(&current_room)
        .and_then(|r| r.npcs.iter().find(|n| n.contains(npc_id)).cloned()) {
        Some(id) => id,
        None => return TapError::NpcNotFound.message(),
    };

    let player = match w.get_player(username) {
        Some(p) => p,
        None => return TapError::PlayerNotFound.message(),
    };

    // A delivery is handed in to its recipient, not to the NPC who gave it out.
    let delivery = player.quests_active.iter()
        .filter_map(|qid| w.quests.get(qid).map(|q| (qid.clone(), q.clone())))
        .find(|(_, q)| q.quest_type == "deliver" && q.target_npc == npc_full_id);

    if let Some((quest_id, quest)) = delivery {
        let (current, required) = quest_progress(player, &quest);
        if current < required {
            return TapError::QuestNotComplete.message();
        }
        return complete_quest(username, &quest_id, &quest, &mut w);
    }

    let quest_id = match w.npcs.get(&npc_full_id).and_then(|n| n.quest.clone()) {
        Some(q) => q,
        None => return TapError::NoQuestAvailable.message(),
    };

    if player.quests_done.contains(&quest_id) {
        return TapError::NoQuestAvailable.message();
    }

    let quest = match w.quests.get(&quest_id) {
        Some(q) => q.clone(),
        None => return TapError::NoQuestAvailable.message(),
    };

    if player.quests_active.contains(&quest_id) {
        // A delivery can only be closed by its recipient.
        if quest.quest_type == "deliver" {
            return TapError::QuestNotComplete.message();
        }
        let (current, required) = quest_progress(player, &quest);
        if current < required {
            return TapError::QuestNotComplete.message();
        }
        return complete_quest(username, &quest_id, &quest, &mut w);
    }

    if let Some(required) = &quest.requires {
        if !player.quests_done.contains(required) {
            return TapError::NoQuestAvailable.message();
        }
    }

    if let Some(p) = w.get_mut_player(username) {
        p.quests_active.push(quest_id.clone());
        // The giver hands over the parcel up front.
        if quest.quest_type == "deliver" {
            for _ in 0..quest.target_count {
                p.inventory.push(quest.target_item.clone());
            }
        }
    }
    tracing::info!(event = "quest_accept", player = %username, quest = %quest_id, kind = %quest.quest_type, "quest accepted");

    match serde_json::to_string(&quest) {
        Ok(json) => format!("OK {}\n", json),
        Err(_) => TapError::SendFailed.message(),
    }
}

pub async fn handle_quests(username: &str, world: &Arc<Mutex<World>>) -> String {
    let w = world.lock().await;
    let player = match w.get_player(username) {
        Some(p) => p,
        None => return TapError::PlayerNotFound.message(),
    };

    let mut quest_list: Vec<serde_json::Value> = Vec::new();

    for qid in &player.quests_active {
        if let Some(quest) = w.quests.get(qid) {
            let (current, required) = quest_progress(player, quest);
            quest_list.push(serde_json::json!({
                "quest_id": qid,
                "status": "active",
                "type": quest.quest_type,
                "progress": format!("{}/{}", current, required),
                "description": quest.description
            }));
        }
    }

    for qid in &player.quests_done {
        if let Some(quest) = w.quests.get(qid) {
            quest_list.push(serde_json::json!({
                "quest_id": qid,
                "status": "completed",
                "description": quest.description
            }));
        } else {
            quest_list.push(serde_json::json!({
                "quest_id": qid,
                "status": "completed"
            }));
        }
    }

    match serde_json::to_string(&quest_list) {
        Ok(json) => format!("OK {}\n", json),
        Err(_) => TapError::SendFailed.message(),
    }
}
