use serde::{Deserialize, Serialize};

use crate::maze::{Difficulty, Maze};

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct InputState {
    pub forward: f32,
    pub strafe: f32,
    pub turn: f32,
    pub pitch: f32,
    pub shoot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Connect { version: u16, username: String },
    Input { sequence: u32, input: InputState },
    Ping(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSnapshot {
    pub id: u64,
    pub username: String,
    pub x: f32,
    pub y: f32,
    pub angle: f32,
    pub score: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ProjectileSnapshot {
    pub id: u64,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    Welcome {
        player_id: u64,
        seed: u64,
        difficulty: Difficulty,
        maze: Maze,
    },
    Snapshot {
        tick: u64,
        players: Vec<PlayerSnapshot>,
        projectiles: Vec<ProjectileSnapshot>,
    },
    Pong(u64),
    Error(String),
}

pub fn encode<T: Serialize>(message: &T) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(message)
}

pub fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, serde_json::Error> {
    serde_json::from_slice(bytes)
}
