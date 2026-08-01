use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Room {
    id: String,
    name: String,
    description: String,
    exits: HashMap<String, String>,
    players: Vec<String>,
    items: Vec<String>,
    npcs: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum PlayerState {
    Alive,
    Dead
}

#[derive(Debug, Clone)]
pub struct Player {
    name: String,
    inventory: Vec<String>,
    quests_active: Vec<String>,
    quests_done: Vec<String>,
    hp: u32,
    status: PlayerState,
    group_id: Option<String>
}

#[derive(Debug, Clone)]
pub struct World {
    players: HashMap<String, Player>,
    rooms: HashMap<String, Room>
}