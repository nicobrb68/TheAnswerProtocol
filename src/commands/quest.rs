use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{World, TapError};

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

    let quest_id = match w.npcs.get(&npc_full_id).and_then(|n| n.quest.clone()) {
        Some(q) => q,
        None => return TapError::NoQuestAvailable.message(),
    };

    let player = match w.get_player(username) {
        Some(p) => p,
        None => return TapError::PlayerNotFound.message(),
    };

    if player.quests_done.contains(&quest_id) {
        return TapError::NoQuestAvailable.message();
    }

    let quest = match w.quests.get(&quest_id) {
        Some(q) => q.clone(),
        None => return TapError::NoQuestAvailable.message(),
    };

    if player.quests_active.contains(&quest_id) {
        let current = player.inventory.iter()
            .filter(|i| *i == &quest.target_item)
            .count() as u32;

        if current < quest.target_count {
            return TapError::QuestNotComplete.message();
        }

        let mut to_remove = quest.target_count;
        if let Some(p) = w.get_mut_player(username) {
            p.inventory.retain(|i| {
                if i == &quest.target_item && to_remove > 0 {
                    to_remove -= 1;
                    false
                } else {
                    true
                }
            });
            for _ in 0..quest.reward_count {
                p.inventory.push(quest.reward.clone());
            }
            p.quests_active.retain(|q| q != &quest_id);
            p.quests_done.push(quest_id.clone());
        }
        tracing::info!(event = "quest_complete", player = %username, quest = %quest_id, reward = %quest.reward, "quest completed");

        return format!("OK {{\"quest_id\": \"{}\", \"status\": \"completed\", \"reward\": \"{}\", \"reward_count\": {}}}\n",
            quest_id, quest.reward, quest.reward_count);
    }

    if let Some(p) = w.get_mut_player(username) { p.quests_active.push(quest_id.clone()); }
    tracing::info!(event = "quest_accept", player = %username, quest = %quest_id, "quest accepted");

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
            let current = player.inventory.iter()
                .filter(|i| *i == &quest.target_item)
                .count() as u32;
            quest_list.push(serde_json::json!({
                "quest_id": qid,
                "status": "active",
                "progress": format!("{}/{}", current, quest.target_count)
            }));
        }
    }

    for qid in &player.quests_done {
        quest_list.push(serde_json::json!({
            "quest_id": qid,
            "status": "completed"
        }));
    }

    match serde_json::to_string(&quest_list) {
        Ok(json) => format!("OK {}\n", json),
        Err(_) => TapError::SendFailed.message(),
    }
}
