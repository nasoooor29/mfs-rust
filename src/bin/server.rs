use std::{
    collections::HashMap,
    env,
    net::{SocketAddr, UdpSocket},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use maze_runner::{
    maze::{empty, generate, Difficulty, Maze},
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
const RESPAWN_TIME: f32 = 0.9;

struct Player {
    id: u64,
    username: String,
    x: f32,
    y: f32,
    angle: f32,
    input: InputState,
    last_seen: Instant,
    shoot_cooldown: f32,
    respawn_timer: f32,
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
    // if any arg have prefix of bind_addr= parse it as the bind addr else default
    let bind_addr = args
        .iter()
        .find_map(|arg| arg.strip_prefix("bind_addr="))
        .unwrap_or(DEFAULT_SERVER_ADDR);
    // if any arg have prefix of difficulty= parse it as the difficulty else default
    let difficulty = args
        .iter()
        .find_map(|arg| arg.strip_prefix("difficulty="))
        .and_then(|s| Difficulty::parse(s))
        .unwrap_or(Difficulty::Medium);
    // if any arg have prefix of seed= parse it as the seed else default
    let seed = args
        .iter()
        .find_map(|arg| arg.strip_prefix("seed="))
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
        });
    // if any arg have empty=1 then generate empty maze else generate random maze
    let is_empty = args.iter().any(|arg| arg == "empty=1");
    let generated: maze_runner::maze::GeneratedMaze;
    if is_empty {
        println!("generating EMPTY maze");
        generated = empty(seed, difficulty);
    } else {
        println!("generating RANDOM maze with seed={seed} difficulty={difficulty:?}");
        generated = generate(seed, difficulty, 8);
    }

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
                            let (x, y) = spawn_position(&generated.maze, next_id);
                            let player = Player {
                                id: next_id,
                                username: clean_name(&username),
                                x,
                                y,
                                angle: 0.0,
                                input: InputState::default(),
                                last_seen: Instant::now(),
                                shoot_cooldown: 0.0,
                                respawn_timer: 0.0,
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
        if player.respawn_timer > 0.0 {
            player.respawn_timer = (player.respawn_timer - dt).max(0.0);
            if player.respawn_timer == 0.0 {
                (player.x, player.y) = respawn_position(maze, player.id);
                player.shoot_cooldown = 0.0;
            }
            continue;
        }

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
                p.respawn_timer == 0.0
                    && p.id != projectile.owner
                    && (p.x - projectile.x).hypot(p.y - projectile.y) < 14.0
            })
            .map(|p| p.id)
        {
            if let Some(victim) = players.values_mut().find(|p| p.id == victim_id) {
                victim.respawn_timer = RESPAWN_TIME;
                victim.shoot_cooldown = 0.0;
                (victim.x, victim.y) = respawn_position(maze, victim.id);
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
            .filter(|p| p.respawn_timer == 0.0)
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

fn spawn_position(maze: &Maze, player_id: u64) -> (f32, f32) {
    let spawn = maze.spawns[(player_id as usize - 1) % maze.spawns.len()];
    maze.world_position(spawn)
}

fn respawn_position(maze: &Maze, player_id: u64) -> (f32, f32) {
    let spawn = maze.spawns[player_id as usize % maze.spawns.len()];
    maze.world_position(spawn)
}

fn send(socket: &UdpSocket, address: SocketAddr, message: &ServerMessage) {
    if let Ok(bytes) = encode(message) {
        let _ = socket.send_to(&bytes, address);
    }
}
