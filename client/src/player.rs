use std::collections::HashSet;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use maze_runner::protocol::{ProjectileSnapshot, ServerMessage};

use crate::{
    minimap::{self, MiniMapRoot, MiniMarker},
    resources::{
        Connection, LocalPlayer, NetPlayer, NetProjectile, SceneAssets, ViewState, WorldCamera,
    },
    ui::ScoreText,
};

#[derive(SystemParam)]
pub struct SnapshotQueries<'w, 's> {
    players: Query<'w, 's, (Entity, &'static NetPlayer, &'static mut Transform)>,
    projectiles: Query<'w, 's, (Entity, &'static NetProjectile)>,
    minimap_root: Query<'w, 's, Entity, With<MiniMapRoot>>,
    minimap_markers: Query<'w, 's, (Entity, &'static MiniMarker)>,
    score_text: Query<'w, 's, &'static mut Text, With<ScoreText>>,
}

pub fn receive_snapshots(
    mut commands: Commands,
    connection: Res<Connection>,
    local: Res<LocalPlayer>,
    assets: Res<SceneAssets>,
    mut queries: SnapshotQueries,
) {
    let mut latest = None;
    let Ok(incoming) = connection.incoming.lock() else {
        return;
    };
    while let Ok(message) = incoming.try_recv() {
        if let ServerMessage::Snapshot {
            players,
            projectiles,
            ..
        } = message
        {
            latest = Some((players, projectiles));
        }
    }
    let Some((snapshots, projectile_snapshots)) = latest else {
        return;
    };
    let player_ids: HashSet<_> = snapshots.iter().map(|p| p.id).collect();
    for (entity, player, _) in &mut queries.players {
        if !player_ids.contains(&player.0) {
            commands.entity(entity).despawn();
        }
    }
    for (entity, marker) in &queries.minimap_markers {
        if !player_ids.contains(&marker.0) {
            commands.entity(entity).despawn();
        }
    }
    let existing_markers: HashSet<_> = queries
        .minimap_markers
        .iter()
        .map(|(_, marker)| marker.0)
        .collect();
    if let Ok(root) = queries.minimap_root.get_single() {
        for snapshot in &snapshots {
            if existing_markers.contains(&snapshot.id) {
                continue;
            }
            commands.entity(root).with_children(|root| {
                minimap::spawn_marker(root, snapshot.id, snapshot.id == local.0);
            });
        }
    }
    for snapshot in &snapshots {
        if let Some((_, _, mut transform)) = queries
            .players
            .iter_mut()
            .find(|(_, player, _)| player.0 == snapshot.id)
        {
            transform.translation = Vec3::new(snapshot.x, 15.0, snapshot.y);
            transform.rotation = Quat::from_rotation_y(-snapshot.angle);
        } else {
            commands.spawn((
                PbrBundle {
                    mesh: assets.player_mesh.clone(),
                    material: if snapshot.id == local.0 {
                        assets.local_player_material.clone()
                    } else {
                        assets.remote_player_material.clone()
                    },
                    transform: Transform::from_xyz(snapshot.x, 15.0, snapshot.y)
                        .with_rotation(Quat::from_rotation_y(-snapshot.angle)),
                    visibility: if snapshot.id == local.0 {
                        Visibility::Hidden
                    } else {
                        Visibility::Visible
                    },
                    ..default()
                },
                NetPlayer(snapshot.id),
            ));
        }
    }
    sync_projectiles(
        &mut commands,
        &assets,
        &queries.projectiles,
        &projectile_snapshots,
    );
    if let Some(me) = snapshots.iter().find(|p| p.id == local.0) {
        if let Ok(mut text) = queries.score_text.get_single_mut() {
            text.sections[0].value = format!(
                "{}  |  score: {}  |  players: {}",
                me.username,
                me.score,
                snapshots.len()
            );
        }
    }
}

fn sync_projectiles(
    commands: &mut Commands,
    assets: &SceneAssets,
    query: &Query<(Entity, &NetProjectile)>,
    snapshots: &[ProjectileSnapshot],
) {
    let ids: HashSet<_> = snapshots.iter().map(|p| p.id).collect();
    for (entity, projectile) in query {
        if !ids.contains(&projectile.0) {
            commands.entity(entity).despawn();
        }
    }
    for snapshot in snapshots {
        if let Some((entity, _)) = query
            .iter()
            .find(|(_, projectile)| projectile.0 == snapshot.id)
        {
            commands
                .entity(entity)
                .insert(Transform::from_xyz(snapshot.x, 14.0, snapshot.y));
        } else {
            commands.spawn((
                PbrBundle {
                    mesh: assets.projectile_mesh.clone(),
                    material: assets.projectile_material.clone(),
                    transform: Transform::from_xyz(snapshot.x, 14.0, snapshot.y),
                    ..default()
                },
                NetProjectile(snapshot.id),
            ));
        }
    }
}

pub fn follow_player(
    local: Res<LocalPlayer>,
    view: Res<ViewState>,
    players: Query<(&NetPlayer, &Transform)>,
    mut camera: Query<&mut Transform, (With<WorldCamera>, Without<NetPlayer>)>,
) {
    if let Some((_, player_transform)) = players.iter().find(|(player, _)| player.0 == local.0) {
        if let Ok(mut camera) = camera.get_single_mut() {
            camera.translation = Vec3::new(
                player_transform.translation.x,
                18.0,
                player_transform.translation.z,
            );
            let horizontal = player_transform.rotation * Vec3::X;
            let direction = Vec3::new(
                horizontal.x * view.pitch.cos(),
                view.pitch.sin(),
                horizontal.z * view.pitch.cos(),
            );
            camera.look_to(direction, Vec3::Y);
        }
    }
}
