use std::collections::HashMap;
use serde::{Deserialize, Serialize};
pub mod commands;

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

impl Player {
    pub fn new(name: String, current_room: String) -> Player{
        Player {
            name,
            inventory: Vec::new(),
            quests_active: Vec::new(),
            quests_done: Vec::new(),
            hp: 100,
            status: PlayerState::Alive,
            current_room,
            group_id: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct World {
    #[serde(default)]
    pub players: HashMap<String, Player>,
    pub rooms: HashMap<String, Room>
}

impl World {
    pub fn add_player(&mut self, player: Player) {
        self.players.insert(player.name.clone(), player);
    }

    pub fn has_player(&self, name: &str) -> bool {
        self.players.contains_key(name)
    }
}
#[derive(Debug)]
pub enum TapError {
    NameInUse,
    NoExit,
    ItemNotFound,
    ItemNotInInventory,
    NpcNotFound,
    NpcNotHostile,
    NoQuestAvailable,
    NotInGroup,
    AlreadyInGroup,
    ConnectionFailed,
    SendFailed,
    NotAuthenticated,
}

impl TapError {
    pub fn message(&self) -> String {
        match self {
            TapError::NameInUse        => "ERR 201 NAME_IN_USE\n".to_string(),
            TapError::NoExit           => "ERR 301 NO_EXIT\n".to_string(),
            TapError::NotInGroup       => "ERR 401 NOT_IN_GROUP\n".to_string(),
            TapError::AlreadyInGroup   => "ERR 402 ALREADY_IN_GROUP\n".to_string(),
            TapError::ItemNotFound     => "ERR 404 ITEM_NOT_FOUND\n".to_string(),
            TapError::ItemNotInInventory => "ERR 404 ITEM_NOT_IN_INVENTORY\n".to_string(),
            TapError::NpcNotFound      => "ERR 404 NPC_NOT_FOUND\n".to_string(),
            TapError::NpcNotHostile    => "ERR 405 NPC_NOT_HOSTILE\n".to_string(),
            TapError::NoQuestAvailable => "ERR 406 NO_QUEST_AVAILABLE\n".to_string(),
            TapError::ConnectionFailed => "ERR 900 CONNECTION_FAILED\n".to_string(),
            TapError::SendFailed       => "ERR 901 SEND_FAILED\n".to_string(),
            TapError::NotAuthenticated => "ERR 000 NOT_AUTHENTICATED\n".to_string(),
        }
    }
}