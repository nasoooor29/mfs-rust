use std::{
    collections::HashSet,
    io::{self, Write},
    net::{SocketAddr, UdpSocket},
    sync::{
        mpsc::{self, Receiver},
        Mutex,
    },
    time::Duration,
};

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PresentMode, PrimaryWindow, WindowResolution},
};
use maze_runner::{
    maze::Maze,
    protocol::{
        decode, encode, ClientMessage, InputState, ProjectileSnapshot, ServerMessage,
        PROTOCOL_VERSION,
    },
    DEFAULT_SERVER_ADDR, PLAYER_SIZE, PROJECTILE_SIZE, TILE_SIZE,
};

#[derive(Resource)]
struct Connection {
    socket: UdpSocket,
    incoming: Mutex<Receiver<ServerMessage>>,
    sequence: u32,
}

#[derive(Resource, Deref)]
struct GameMaze(Maze);

#[derive(Resource)]
struct LocalPlayer(u64);

#[derive(Resource)]
struct Controls {
    forward: KeyCode,
    backward: KeyCode,
    left: KeyCode,
    right: KeyCode,
    turn_left: KeyCode,
    turn_right: KeyCode,
    look_up: KeyCode,
    look_down: KeyCode,
    shoot: KeyCode,
}

#[derive(Resource, Default)]
struct ViewState {
    pitch: f32,
    mouse_turn: f32,
}

#[derive(Resource)]
struct SceneAssets {
    player_mesh: Handle<Mesh>,
    projectile_mesh: Handle<Mesh>,
    local_player_material: Handle<StandardMaterial>,
    remote_player_material: Handle<StandardMaterial>,
    projectile_material: Handle<StandardMaterial>,
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
struct WorldCamera;
#[derive(Component)]
struct NetPlayer(u64);
#[derive(Component)]
struct NetProjectile(u64);
#[derive(Component)]
struct MiniMapRoot;
#[derive(Component)]
struct MiniMarker(u64);
#[derive(Component)]
struct FpsText;
#[derive(Component)]
struct ScoreText;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server_text = prompt("Server address", DEFAULT_SERVER_ADDR)?;
    let server: SocketAddr = server_text.parse()?;
    let username = prompt("Username", "Player")?;
    println!("Connecting...");
    let (socket, welcome) = connect(server, username)?;
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

    let (sender, receiver) = mpsc::channel();
    let receive_socket = socket.try_clone()?;
    std::thread::spawn(move || {
        let mut buffer = [0u8; 65_507];
        loop {
            match receive_socket.recv(&mut buffer) {
                Ok(length) => {
                    if let Ok(message) = decode(&buffer[..length]) {
                        if sender.send(message).is_err() {
                            break;
                        }
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2))
                }
                Err(_) => break,
            }
        }
    });

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
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                update_mouse_capture,
                update_view_input,
                send_input,
                receive_snapshots,
                follow_player,
                update_minimap,
                update_ui,
            )
                .chain(),
        )
        .run();
    Ok(())
}

fn prompt(label: &str, default: &str) -> io::Result<String> {
    print!("{label} [{default}]: ");
    io::stdout().flush()?;
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    let value = value.trim();
    Ok(if value.is_empty() {
        default.into()
    } else {
        value.into()
    })
}

fn connect(
    server: SocketAddr,
    username: String,
) -> Result<(UdpSocket, ServerMessage), Box<dyn std::error::Error>> {
    let bind = if server.is_ipv6() {
        "[::]:0"
    } else {
        "0.0.0.0:0"
    };
    let socket = UdpSocket::bind(bind)?;
    socket.connect(server)?;
    socket.set_read_timeout(Some(Duration::from_secs(1)))?;
    let request = encode(&ClientMessage::Connect {
        version: PROTOCOL_VERSION,
        username,
    })?;
    let mut buffer = [0u8; 65_507];
    for _ in 0..5 {
        socket.send(&request)?;
        if let Ok(length) = socket.recv(&mut buffer) {
            let message: ServerMessage = decode(&buffer[..length])?;
            match message {
                ServerMessage::Welcome { .. } => {
                    socket.set_read_timeout(None)?;
                    socket.set_nonblocking(true)?;
                    return Ok((socket, message));
                }
                ServerMessage::Error(reason) => return Err(reason.into()),
                _ => {}
            }
        }
    }
    Err("server did not respond after 5 attempts".into())
}

fn setup(
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
            shadows_enabled: true,
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
            let (world_x, world_y) = maze.world_position(maze_runner::maze::TilePos { x, y });
            commands.spawn(PbrBundle {
                mesh: wall_mesh.clone(),
                material: wall_material.clone(),
                transform: Transform::from_xyz(world_x, 26.0, world_y),
                ..default()
            });
        }
    }
    // NOTE: removing the orange guy
    // let (objective_x, objective_y) = maze.world_position(maze.objective);
    // commands.spawn(PbrBundle {
    //     mesh: meshes.add(Cuboid::new(12.0, 30.0, 12.0)),
    //     material: materials.add(Color::srgb(1.0, 0.62, 0.08)),
    //     transform: Transform::from_xyz(objective_x, 15.0, objective_y),
    //     ..default()
    // });

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

    commands.spawn((
        TextBundle::from_sections([TextSection::new(
            "FPS: --",
            TextStyle {
                font_size: 18.0,
                color: Color::WHITE,
                ..default()
            },
        )])
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            top: Val::Px(8.0),
            ..default()
        }),
        FpsText,
    ));
    commands.spawn((
        TextBundle::from_section(
            "Waiting for snapshot",
            TextStyle {
                font_size: 18.0,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            top: Val::Px(34.0),
            ..default()
        }),
        ScoreText,
    ));
    commands.spawn(
        TextBundle::from_section(
            "WASD move  |  mouse/arrow keys look  |  SPACE shoot  |  ESC release mouse",
            TextStyle {
                font_size: 16.0,
                color: Color::srgb(0.75, 0.8, 0.82),
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            bottom: Val::Px(8.0),
            ..default()
        }),
    );
    commands.spawn(NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            width: Val::Px(4.0),
            height: Val::Px(4.0),
            ..default()
        },
        background_color: Color::WHITE.into(),
        transform: Transform::from_xyz(-2.0, -2.0, 0.0),
        z_index: ZIndex::Global(20),
        ..default()
    });

    let cell = 3.0;
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    right: Val::Px(12.0),
                    top: Val::Px(12.0),
                    width: Val::Px(maze.width as f32 * cell),
                    height: Val::Px(maze.height as f32 * cell),
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
                        Color::srgb(0.28, 0.31, 0.33)
                    } else {
                        Color::srgb(0.05, 0.07, 0.08)
                    };
                    root.spawn(NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            left: Val::Px(x as f32 * cell),
                            top: Val::Px((maze.height - 1 - y) as f32 * cell),
                            width: Val::Px(cell),
                            height: Val::Px(cell),
                            ..default()
                        },
                        background_color: color.into(),
                        ..default()
                    });
                }
            }
            root.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        width: Val::Px(5.0),
                        height: Val::Px(5.0),
                        ..default()
                    },
                    background_color: Color::srgb(1.0, 0.25, 0.2).into(),
                    z_index: ZIndex::Local(2),
                    ..default()
                },
                MiniMarker(local.0),
            ));
        });
}

fn update_mouse_capture(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Ok(mut window) = window.get_single_mut() else {
        return;
    };
    if keys.just_pressed(KeyCode::Escape) {
        window.cursor.grab_mode = CursorGrabMode::None;
        window.cursor.visible = true;
    } else if mouse.just_pressed(MouseButton::Left) {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
    }
}

fn update_view_input(
    mut motion: EventReader<MouseMotion>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    controls: Res<Controls>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut view: ResMut<ViewState>,
) {
    let mouse_delta = motion
        .read()
        .fold(Vec2::ZERO, |sum, event| sum + event.delta);
    let captured = window
        .get_single()
        .is_ok_and(|window| !window.cursor.visible);
    view.mouse_turn = if captured {
        (mouse_delta.x * 0.025).clamp(-1.0, 1.0)
    } else {
        0.0
    };

    let keyboard_pitch =
        f32::from(keys.pressed(controls.look_up)) - f32::from(keys.pressed(controls.look_down));
    let mouse_pitch = if captured {
        -mouse_delta.y * 0.0025
    } else {
        0.0
    };
    view.pitch =
        (view.pitch + keyboard_pitch * time.delta_seconds() * 1.5 + mouse_pitch).clamp(-1.35, 1.35);
}

fn send_input(
    keys: Res<ButtonInput<KeyCode>>,
    controls: Res<Controls>,
    view: Res<ViewState>,
    mut connection: ResMut<Connection>,
) {
    let axis =
        |positive, negative| f32::from(keys.pressed(positive)) - f32::from(keys.pressed(negative));
    let input = InputState {
        forward: axis(controls.forward, controls.backward),
        strafe: axis(controls.right, controls.left),
        turn: (axis(controls.turn_right, controls.turn_left) + view.mouse_turn).clamp(-1.0, 1.0),
        pitch: axis(controls.look_up, controls.look_down),
        shoot: keys.pressed(controls.shoot),
    };
    connection.sequence = connection.sequence.wrapping_add(1);
    if let Ok(bytes) = encode(&ClientMessage::Input {
        sequence: connection.sequence,
        input,
    }) {
        let _ = connection.socket.send(&bytes);
    }
}

fn receive_snapshots(
    mut commands: Commands,
    connection: Res<Connection>,
    local: Res<LocalPlayer>,
    assets: Res<SceneAssets>,
    mut players: Query<(Entity, &NetPlayer, &mut Transform)>,
    projectiles: Query<(Entity, &NetProjectile)>,
    minimap_root: Query<Entity, With<MiniMapRoot>>,
    minimap_markers: Query<(Entity, &MiniMarker)>,
    mut score_text: Query<&mut Text, With<ScoreText>>,
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
    for (entity, player, _) in &mut players {
        if !player_ids.contains(&player.0) {
            commands.entity(entity).despawn();
        }
    }
    for (entity, marker) in &minimap_markers {
        if !player_ids.contains(&marker.0) {
            commands.entity(entity).despawn();
        }
    }
    let existing_markers: HashSet<_> = minimap_markers.iter().map(|(_, marker)| marker.0).collect();
    if let Ok(root) = minimap_root.get_single() {
        for snapshot in &snapshots {
            if existing_markers.contains(&snapshot.id) {
                continue;
            }
            let (size, color) = if snapshot.id == local.0 {
                (5.0, Color::srgb(1.0, 0.25, 0.2))
            } else {
                (4.0, Color::srgb(0.35, 0.75, 1.0))
            };
            commands.entity(root).with_children(|root| {
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
                    MiniMarker(snapshot.id),
                ));
            });
        }
    }
    for snapshot in &snapshots {
        if let Some((_, _, mut transform)) = players
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
    sync_projectiles(&mut commands, &assets, &projectiles, &projectile_snapshots);
    if let Some(me) = snapshots.iter().find(|p| p.id == local.0) {
        if let Ok(mut text) = score_text.get_single_mut() {
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

fn follow_player(
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

fn update_minimap(
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
        style.left = Val::Px(tile_x * 3.0 - 1.0);
        style.top = Val::Px((maze.height as f32 - tile_y) * 3.0 - 1.0);
    }
}

fn update_ui(diagnostics: Res<DiagnosticsStore>, mut text: Query<&mut Text, With<FpsText>>) {
    if let Some(fps) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
    {
        if let Ok(mut text) = text.get_single_mut() {
            text.sections[0].value = format!("FPS: {fps:.0}");
        }
    }
}
