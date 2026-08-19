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

    let player = w.get_player(username).unwrap();
    if player.quests_active.contains(&quest_id) {
        return TapError::NoQuestAvailable.message();
    }
    if player.quests_done.contains(&quest_id) {
        return TapError::NoQuestAvailable.message();
    }

    let quest = match w.quests.get(&quest_id) {
        Some(q) => q.clone(),
        None => return TapError::NoQuestAvailable.message(),
    };

    w.get_mut_player(username).unwrap().quests_active.push(quest_id);

    format!("OK {}\n", serde_json::to_string(&quest).unwrap())
}