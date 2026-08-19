use std::collections::HashMap;
use serde::{Deserialize, Serialize};

pub mod commands;
pub mod utils;
pub mod events;

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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Quest {
    pub id: String,
    pub description: String,
    pub giver: String,
    #[serde(rename = "type")]
    pub quest_type: String,
    pub target_item: String,
    pub target_count: u32,
    pub reward: String,
    pub reward_count: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Player {
    pub name: String,
    pub inventory: Vec<String>,
    pub quests_active: Vec<String>,
    pub quests_done: Vec<String>,
    pub hp: u32,
    pub max_hp: u32,
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
            max_hp: 100,
            status: PlayerState::Alive,
            current_room,
            group_id: None,
        }
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Group {
    pub id: String,
    pub leader: String,
    pub players: Vec<String>,
    pub invited: Vec<String>
}

impl Group {
    pub fn new(id: String, username: String) -> Group {
        Group {
            id,
            leader: username.clone(),
            players: vec![username],
            invited: Vec::new()
        }
    }

    pub fn add_player(&mut self, player: String) {
        self.players.push(player);
    }

}

#[derive(Debug, Clone, Deserialize)]
pub struct World {
    #[serde(default)]
    pub players: HashMap<String, Player>,
    pub rooms: HashMap<String, Room>,
    pub spawn: String,
    #[serde(default)]
    pub groups: HashMap<String, Group>,
    #[serde(default)]
    pub npcs: HashMap<String, Npc>,
    #[serde(default)]
    pub items: HashMap<String, Item>,
    pub quests: HashMap<String, Quest>
}

impl World {

    // Player impl

    pub fn add_player(&mut self, player: Player) {
        self.players.insert(player.name.clone(), player);
    }

    pub fn get_mut_player(&mut self, name: &str) -> Option<&mut Player> {
        self.players.get_mut(name)
    }

    pub fn get_player(&self, name: &str) -> Option<&Player> {
        self.players.get(name)
    }


    pub fn has_player(&self, name: &str) -> bool {
        self.players.contains_key(name)
    }

    // Room impl

    pub fn get_room(&self, id: &str) -> Option<&Room> {
        self.rooms.get(id)
    }

    pub fn get_mut_room(&mut self, id: &str) -> Option<&mut Room> {
        self.rooms.get_mut(id)
    }


    // Group impl

    pub fn add_group(&mut self, group: Group) {
        self.groups.insert(group.id.clone(), group);
    }

    pub fn has_group(&self, id: &str) -> bool {
        self.groups.contains_key(id)
    }

    pub fn get_group(&self, id: &str) -> Option<&Group> { self.groups.get(id) }

    pub fn get_mut_group(&mut self, id: &str) -> Option<&mut Group> { self.groups.get_mut(id) }
    
    // ncp impl

    pub fn get_npc(&self, id: &str) -> Option<&Npc> {
        self.npcs.get(id)
    }

    pub fn get_mut_npc(&mut self, id: &str) -> Option<&mut Npc> {
        self.npcs.get_mut(id)
    }

}
#[derive(Debug)]
pub enum TapError {
    NameInUse,
    InvalidUsername,
    GroupNameInUse,
    InvalidGroupName,
    NoExit,
    ItemNotFound,
    PlayerNotFound,
    GroupNotFound,
    ItemNotInInventory,
    NotGroupLeader,
    NpcNotFound,
    NpcNotHostile,
    NoQuestAvailable,
    NotInGroup,
    NotInvited,
    PlayerAlreadyInGroup,
    CannotInviteSelf,
    AlreadyInGroup,
    ConnectionFailed,
    SendFailed,
    NotAuthenticated,
}

impl TapError {
    pub fn message(&self) -> String {
        match self {
            TapError::NameInUse        => "ERR 201 NAME_IN_USE\n".to_string(),
            TapError::InvalidUsername  => "ERR 202 INVALID_USER_NAME\n".to_string(),
            TapError::GroupNameInUse   => "ERR 211 GROUP_NAME_IN_USE\n".to_string(),
            TapError::InvalidGroupName => "ERR 212 INVALID_GROUP_NAME\n".to_string(),
            TapError::NoExit           => "ERR 301 NO_EXIT\n".to_string(),
            TapError::NotInGroup       => "ERR 401 NOT_IN_GROUP\n".to_string(),
            TapError::NotInvited => "ERR 407 NOT_INVITED\n".to_string(),
            TapError::AlreadyInGroup   => "ERR 402 ALREADY_IN_GROUP\n".to_string(),
            TapError::PlayerAlreadyInGroup   => "ERR 402 PLAYER_ALREADY_IN_GROUP\n".to_string(),
            TapError::NotGroupLeader => "ERR 403 NOT_GROUP_LEADER\n".to_string(),
            TapError::PlayerNotFound   => "ERR 404 PLAYER_NOT_FOUND\n".to_string(),
            TapError::GroupNotFound    => "ERR 404 GROUP_NOT_FOUND\n".to_string(),
            TapError::ItemNotFound     => "ERR 404 ITEM_NOT_FOUND\n".to_string(),
            TapError::ItemNotInInventory => "ERR 404 ITEM_NOT_IN_INVENTORY\n".to_string(),
            TapError::NpcNotFound      => "ERR 404 NPC_NOT_FOUND\n".to_string(),
            TapError::NpcNotHostile    => "ERR 405 NPC_NOT_HOSTILE\n".to_string(),
            TapError::NoQuestAvailable => "ERR 406 NO_QUEST_AVAILABLE\n".to_string(),
            TapError::CannotInviteSelf => "ERR 407 CANNOT_INVITE_SELF\n".to_string(),
            TapError::ConnectionFailed => "ERR 900 CONNECTION_FAILED\n".to_string(),
            TapError::SendFailed       => "ERR 901 SEND_FAILED\n".to_string(),
            TapError::NotAuthenticated => "ERR 000 NOT_AUTHENTICATED\n".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Npc {
    pub id: String,
    pub name: String,
    pub dialogue: Vec<String>,
    pub hostile: bool,
    pub hp: Option<u32>,
    pub damage: Option<u32>,
    pub quest: Option<String>
}


#[derive(Debug, Clone, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub damage: Option<u32>,
    pub armor: Option<u32>,
    pub heal: Option<u32>,
}