use bevy::{
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use maze_runner::{maze::TilePos, PLAYER_SIZE, PROJECTILE_SIZE, TILE_SIZE};

use crate::{
    minimap,
    resources::{GameMaze, LocalPlayer, SceneAssets, WorldCamera},
    ui,
};

pub fn setup(
    mut commands: Commands,
    maze: Res<GameMaze>,
    local: Res<LocalPlayer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
) {
    if let Ok(mut window) = window.get_single_mut() {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
    }

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 18.0, 0.0).looking_to(Vec3::X, Vec3::Y),
            ..default()
        },
        WorldCamera,
    ));
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.42, 0.48, 0.55),
        brightness: 230.0,
    });
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 4_000.0,
            shadows_enabled: false,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, -0.6, 0.0)),
        ..default()
    });

    let wall_mesh = meshes.add(Cuboid::new(TILE_SIZE, 52.0, TILE_SIZE));
    let floor_mesh = meshes.add(Cuboid::new(
        maze.width as f32 * TILE_SIZE,
        2.0,
        maze.height as f32 * TILE_SIZE,
    ));
    let wall_material = materials.add(Color::srgb(0.16, 0.38, 0.44));
    let floor_material = materials.add(Color::srgb(0.075, 0.09, 0.105));
    commands.spawn(PbrBundle {
        mesh: floor_mesh,
        material: floor_material,
        transform: Transform::from_xyz(0.0, -1.0, 0.0),
        ..default()
    });
    for y in 0..maze.height {
        for x in 0..maze.width {
            if maze.tiles[y * maze.width + x] == 1 {
                continue;
            }
            let (world_x, world_y) = maze.world_position(TilePos { x, y });
            commands.spawn(PbrBundle {
                mesh: wall_mesh.clone(),
                material: wall_material.clone(),
                transform: Transform::from_xyz(world_x, 26.0, world_y),
                ..default()
            });
        }
    }

    commands.insert_resource(SceneAssets {
        player_mesh: meshes.add(Cuboid::new(
            PLAYER_SIZE * 0.56,
            PLAYER_SIZE * 0.94,
            PLAYER_SIZE * 0.56,
        )),
        projectile_mesh: meshes.add(Sphere::new(PROJECTILE_SIZE * 0.5)),
        local_player_material: materials.add(Color::srgb(0.95, 0.28, 0.20)),
        remote_player_material: materials.add(Color::srgb(0.35, 0.75, 1.0)),
        projectile_material: materials.add(Color::srgb(1.0, 0.8, 0.15)),
    });

    ui::spawn_ui(&mut commands);
    minimap::spawn_minimap(&mut commands, &maze, &local);
}
