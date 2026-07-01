use std::{net::SocketAddr, sync::Mutex};

use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin,
    prelude::*,
    window::{PresentMode, WindowResolution},
};
use maze_runner::{protocol::ServerMessage, DEFAULT_SERVER_ADDR};

mod connection;
mod input;
mod minimap;
mod player;
mod resources;
mod setup;
mod ui;

use resources::{Connection, Controls, GameMaze, LocalPlayer, ViewState};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server_text = connection::prompt("Server address", DEFAULT_SERVER_ADDR)?;
    let server: SocketAddr = server_text.parse()?;
    let username = connection::prompt("Username", "Player")?;
    println!("Connecting...");
    let (socket, welcome) = connection::connect(server, username)?;
    let ServerMessage::Welcome {
        player_id,
        seed,
        difficulty,
        maze,
    } = welcome
    else {
        unreachable!()
    };
    println!("Connected. maze seed={seed}, difficulty={difficulty:?}");

    let receiver = connection::spawn_receiver(&socket)?;

    App::new()
        .insert_resource(ClearColor(Color::srgb(0.025, 0.03, 0.04)))
        .insert_resource(GameMaze(maze))
        .insert_resource(LocalPlayer(player_id))
        .insert_resource(Controls::default())
        .insert_resource(ViewState::default())
        .insert_resource(Connection {
            socket,
            incoming: Mutex::new(receiver),
            sequence: 0,
        })
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Maze Runner".into(),
                        resolution: WindowResolution::new(1280.0, 720.0),
                        present_mode: PresentMode::AutoVsync,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_systems(Startup, setup::setup)
        .add_systems(
            Update,
            (
                input::update_mouse_capture,
                input::update_view_input,
                input::send_input,
                player::receive_snapshots,
                player::follow_player,
                minimap::update_minimap,
                ui::update_ui,
            )
                .chain(),
        )
        .run();
    Ok(())
}
