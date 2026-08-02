use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub description: String,
    pub exits: HashMap<String, String>,
    pub players: Vec<String>,
    pub items: Vec<String>,
    pub npcs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub enum PlayerState {
    Alive,
    Dead
}

#[derive(Debug, Clone, Deserialize)]
pub struct Player {
    pub name: String,
    pub inventory: Vec<String>,
    pub quests_active: Vec<String>,
    pub quests_done: Vec<String>,
    pub hp: u32,
    pub status: PlayerState,
    pub current_room: String,
    pub group_id: Option<String>
}

#[derive(Debug, Clone, Deserialize)]
pub struct World {
    #[serde(default)]
    pub players: HashMap<String, Player>,
    pub rooms: HashMap<String, Room>
}