use std::collections::HashMap;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Room {
    id: String,
    name: String,
    description: String,
    exits: HashMap<String, String>,
    players: Vec<String>,
    items: Vec<String>,
    npcs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub enum PlayerState {
    Alive,
    Dead
}

#[derive(Debug, Clone, Deserialize)]
pub struct Player {
    name: String,
    inventory: Vec<String>,
    quests_active: Vec<String>,
    quests_done: Vec<String>,
    hp: u32,
    status: PlayerState,
    group_id: Option<String>
}

#[derive(Debug, Clone, Deserialize)]
pub struct World {
    #[serde(default)]
    players: HashMap<String, Player>,
    rooms: HashMap<String, Room>
}