use std::{
    net::UdpSocket,
    sync::{mpsc::Receiver, Mutex},
};

use bevy::prelude::*;
use maze_runner::maze::Maze;
use maze_runner::protocol::ServerMessage;

#[derive(Resource)]
pub struct Connection {
    pub socket: UdpSocket,
    pub incoming: Mutex<Receiver<ServerMessage>>,
    pub sequence: u32,
}

#[derive(Resource, Deref)]
pub struct GameMaze(pub Maze);

#[derive(Resource)]
pub struct LocalPlayer(pub u64);

#[derive(Resource)]
pub struct Controls {
    pub forward: KeyCode,
    pub backward: KeyCode,
    pub left: KeyCode,
    pub right: KeyCode,
    pub turn_left: KeyCode,
    pub turn_right: KeyCode,
    pub look_up: KeyCode,
    pub look_down: KeyCode,
    pub shoot: KeyCode,
}

#[derive(Resource, Default)]
pub struct ViewState {
    pub pitch: f32,
    pub mouse_turn: f32,
}

#[derive(Resource)]
pub struct SceneAssets {
    pub player_mesh: Handle<Mesh>,
    pub projectile_mesh: Handle<Mesh>,
    pub local_player_material: Handle<StandardMaterial>,
    pub remote_player_material: Handle<StandardMaterial>,
    pub projectile_material: Handle<StandardMaterial>,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            forward: KeyCode::KeyW,
            backward: KeyCode::KeyS,
            left: KeyCode::KeyA,
            right: KeyCode::KeyD,
            turn_left: KeyCode::ArrowLeft,
            turn_right: KeyCode::ArrowRight,
            look_up: KeyCode::ArrowUp,
            look_down: KeyCode::ArrowDown,
            shoot: KeyCode::Space,
        }
    }
}

#[derive(Component)]
pub struct WorldCamera;

#[derive(Component)]
pub struct NetPlayer(pub u64);

#[derive(Component)]
pub struct NetProjectile(pub u64);
