use std::{
    collections::HashMap,
    env,
    net::{SocketAddr, UdpSocket},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use maze_runner::{
    maze::{generate, Difficulty, Maze},
    protocol::{
        decode, encode, ClientMessage, InputState, PlayerSnapshot, ProjectileSnapshot,
        ServerMessage, PROTOCOL_VERSION,
    },
    DEFAULT_SERVER_ADDR,
};

const TICK_RATE: f32 = 60.0;
const PLAYER_SPEED: f32 = 150.0;
const TURN_SPEED: f32 = 2.8;
const PROJECTILE_SPEED: f32 = 360.0;
const MAX_CLIENTS: usize = 64;

struct Player {
    id: u64,
    username: String,
    x: f32,
    y: f32,
    angle: f32,
    input: InputState,
    last_seen: Instant,
    shoot_cooldown: f32,
    score: u32,
}

struct Projectile {
    id: u64,
    owner: u64,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    lifetime: f32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let bind_addr = args
        .get(1)
        .map(String::as_str)
        .unwrap_or(DEFAULT_SERVER_ADDR);
    let difficulty = args
        .get(2)
        .and_then(|s| Difficulty::parse(s))
        .unwrap_or(Difficulty::Medium);
    let seed = args.get(3).and_then(|s| s.parse().ok()).unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    });
    let generated = generate(seed, difficulty, 8);
    println!(
        "maze seed={} difficulty={difficulty:?} fallback={}",
        generated.effective_seed, generated.used_fallback
    );

    let socket = UdpSocket::bind(bind_addr)?;
    socket.set_nonblocking(true)?;
    println!("server listening on {bind_addr} (up to {MAX_CLIENTS} players)");

    let mut players: HashMap<SocketAddr, Player> = HashMap::new();
    let mut projectiles = Vec::new();
    let mut next_id = 1u64;
    let mut tick = 0u64;
    let tick_duration = Duration::from_secs_f32(1.0 / TICK_RATE);
    let mut next_tick = Instant::now();
    let mut buffer = [0u8; 65_507];

    loop {
        while let Ok((length, address)) = socket.recv_from(&mut buffer) {
            let Ok(message) = decode::<ClientMessage>(&buffer[..length]) else {
                continue;
            };
            match message {
                ClientMessage::Connect { version, username } => {
                    if version != PROTOCOL_VERSION {
                        send(
                            &socket,
                            address,
                            &ServerMessage::Error("client/server version mismatch".into()),
                        );
                    } else if !players.contains_key(&address) && players.len() >= MAX_CLIENTS {
                        send(
                            &socket,
                            address,
                            &ServerMessage::Error("server is full".into()),
                        );
                    } else {
                        let player = players.entry(address).or_insert_with(|| {
                            let spawn = generated.maze.spawns
                                [(next_id as usize - 1) % generated.maze.spawns.len()];
                            let (x, y) = generated.maze.world_position(spawn);
                            let player = Player {
                                id: next_id,
                                username: clean_name(&username),
                                x,
                                y,
                                angle: 0.0,
                                input: InputState::default(),
                                last_seen: Instant::now(),
                                shoot_cooldown: 0.0,
                                score: 0,
                            };
                            next_id += 1;
                            player
                        });
                        player.last_seen = Instant::now();
                        send(
                            &socket,
                            address,
                            &ServerMessage::Welcome {
                                player_id: player.id,
                                seed: generated.effective_seed,
                                difficulty,
                                maze: generated.maze.clone(),
                            },
                        );
                        println!("{} joined from {address}", player.username);
                    }
                }
                ClientMessage::Input { input, .. } => {
                    if let Some(player) = players.get_mut(&address) {
                        player.input = input;
                        player.last_seen = Instant::now();
                    }
                }
                ClientMessage::Ping(value) => send(&socket, address, &ServerMessage::Pong(value)),
            }
        }

        let now = Instant::now();
        if now >= next_tick {
            update_players(
                &generated.maze,
                &mut players,
                &mut projectiles,
                &mut next_id,
            );
            update_projectiles(&generated.maze, &mut players, &mut projectiles);
            players
                .retain(|_, player| now.duration_since(player.last_seen) < Duration::from_secs(10));
            tick += 1;
            if tick % 3 == 0 {
                broadcast_snapshot(&socket, &players, &projectiles, tick);
            }
            next_tick += tick_duration;
            if next_tick < now {
                next_tick = now + tick_duration;
            }
        } else {
            thread::sleep((next_tick - now).min(Duration::from_millis(2)));
        }
    }
}

fn clean_name(name: &str) -> String {
    let clean: String = name.chars().filter(|c| !c.is_control()).take(20).collect();
    if clean.trim().is_empty() {
        "Player".into()
    } else {
        clean
    }
}

fn update_players(
    maze: &Maze,
    players: &mut HashMap<SocketAddr, Player>,
    projectiles: &mut Vec<Projectile>,
    next_id: &mut u64,
) {
    let dt = 1.0 / TICK_RATE;
    for player in players.values_mut() {
        player.angle += player.input.turn.clamp(-1.0, 1.0) * TURN_SPEED * dt;
        let (sin, cos) = player.angle.sin_cos();
        let forward = player.input.forward.clamp(-1.0, 1.0);
        let strafe = player.input.strafe.clamp(-1.0, 1.0);
        let length = (forward * forward + strafe * strafe).sqrt().max(1.0);
        let dx = (cos * forward - sin * strafe) / length * PLAYER_SPEED * dt;
        let dy = (sin * forward + cos * strafe) / length * PLAYER_SPEED * dt;
        if maze.world_is_clear(player.x + dx, player.y, 13.5) {
            player.x += dx;
        }
        if maze.world_is_clear(player.x, player.y + dy, 13.5) {
            player.y += dy;
        }
        player.shoot_cooldown = (player.shoot_cooldown - dt).max(0.0);
        if player.input.shoot && player.shoot_cooldown == 0.0 {
            projectiles.push(Projectile {
                id: *next_id,
                owner: player.id,
                x: player.x + cos * 18.0,
                y: player.y + sin * 18.0,
                vx: cos * PROJECTILE_SPEED,
                vy: sin * PROJECTILE_SPEED,
                lifetime: 2.5,
            });
            *next_id += 1;
            player.shoot_cooldown = 0.28;
        }
    }
}

fn update_projectiles(
    maze: &Maze,
    players: &mut HashMap<SocketAddr, Player>,
    projectiles: &mut Vec<Projectile>,
) {
    let dt = 1.0 / TICK_RATE;
    for projectile in projectiles.iter_mut() {
        projectile.x += projectile.vx * dt;
        projectile.y += projectile.vy * dt;
        projectile.lifetime -= dt;
        if !maze.world_is_clear(projectile.x, projectile.y, 4.0) {
            projectile.lifetime = 0.0;
            continue;
        }
        if let Some(victim_id) = players
            .values()
            .find(|p| {
                p.id != projectile.owner && (p.x - projectile.x).hypot(p.y - projectile.y) < 14.0
            })
            .map(|p| p.id)
        {
            if let Some(victim) = players.values_mut().find(|p| p.id == victim_id) {
                let spawn = maze.spawns[victim.id as usize % maze.spawns.len()];
                (victim.x, victim.y) = maze.world_position(spawn);
            }
            if let Some(owner) = players.values_mut().find(|p| p.id == projectile.owner) {
                owner.score += 1;
            }
            projectile.lifetime = 0.0;
        }
    }
    projectiles.retain(|p| p.lifetime > 0.0);
}

fn broadcast_snapshot(
    socket: &UdpSocket,
    players: &HashMap<SocketAddr, Player>,
    projectiles: &[Projectile],
    tick: u64,
) {
    let message = ServerMessage::Snapshot {
        tick,
        players: players
            .values()
            .map(|p| PlayerSnapshot {
                id: p.id,
                username: p.username.clone(),
                x: p.x,
                y: p.y,
                angle: p.angle,
                score: p.score,
            })
            .collect(),
        projectiles: projectiles
            .iter()
            .map(|p| ProjectileSnapshot {
                id: p.id,
                x: p.x,
                y: p.y,
            })
            .collect(),
    };
    if let Ok(bytes) = encode(&message) {
        for address in players.keys() {
            let _ = socket.send_to(&bytes, address);
        }
    }
}

fn send(socket: &UdpSocket, address: SocketAddr, message: &ServerMessage) {
    if let Ok(bytes) = encode(message) {
        let _ = socket.send_to(&bytes, address);
    }
}
