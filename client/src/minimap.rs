use bevy::prelude::*;
use maze_runner::TILE_SIZE;

use crate::resources::{GameMaze, LocalPlayer, NetPlayer};

pub const CELL_SIZE: f32 = 3.0;

#[derive(Component)]
pub struct MiniMapRoot;

#[derive(Component)]
pub struct MiniMarker(pub u64);

pub fn spawn_minimap(commands: &mut Commands, maze: &GameMaze, local: &LocalPlayer) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    right: Val::Px(12.0),
                    top: Val::Px(12.0),
                    width: Val::Px(maze.width as f32 * CELL_SIZE),
                    height: Val::Px(maze.height as f32 * CELL_SIZE),
                    ..default()
                },
                background_color: Color::BLACK.into(),
                z_index: ZIndex::Global(10),
                ..default()
            },
            MiniMapRoot,
        ))
        .with_children(|root| {
            for y in 0..maze.height {
                for x in 0..maze.width {
                    let color = if maze.tiles[y * maze.width + x] == 1 {
                        Color::srgb(0.05, 0.07, 0.08)
                    } else {
                        Color::srgb(0.28, 0.31, 0.33)
                    };
                    root.spawn(NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            left: Val::Px(x as f32 * CELL_SIZE),
                            top: Val::Px((maze.height - 1 - y) as f32 * CELL_SIZE),
                            width: Val::Px(CELL_SIZE),
                            height: Val::Px(CELL_SIZE),
                            ..default()
                        },
                        background_color: color.into(),
                        ..default()
                    });
                }
            }
            spawn_marker(root, local.0, true);
        });
}

pub fn spawn_marker(root: &mut ChildBuilder, player_id: u64, is_local: bool) {
    let (size, color) = if is_local {
        (5.0, Color::srgb(1.0, 0.25, 0.2))
    } else {
        (4.0, Color::srgb(0.35, 0.75, 1.0))
    };
    root.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Px(size),
                height: Val::Px(size),
                ..default()
            },
            background_color: color.into(),
            z_index: ZIndex::Local(2),
            ..default()
        },
        MiniMarker(player_id),
    ));
}

pub fn update_minimap(
    maze: Res<GameMaze>,
    players: Query<(&NetPlayer, &Transform)>,
    mut markers: Query<(&MiniMarker, &mut Style)>,
) {
    for (marker, mut style) in &mut markers {
        let Some((_, transform)) = players.iter().find(|(player, _)| player.0 == marker.0) else {
            continue;
        };
        let tile_x = transform.translation.x / TILE_SIZE + maze.width as f32 / 2.0;
        let tile_y = transform.translation.z / TILE_SIZE + maze.height as f32 / 2.0;
        style.left = Val::Px(tile_x * CELL_SIZE - 1.0);
        style.top = Val::Px((maze.height as f32 - tile_y) * CELL_SIZE - 1.0);
    }
}
