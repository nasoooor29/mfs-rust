use std::{
    collections::HashMap,
    net::UdpSocket,
    time::{Duration, Instant},
};

use common::*;
use postcard::{from_bytes, to_stdvec};

fn main() -> std::io::Result<()> {
    let sock = UdpSocket::bind("0.0.0.0:34254")?;
    sock.set_nonblocking(true)?;

    let mut next_id: ClientId = 1;
    let mut clients: HashMap<std::net::SocketAddr, ClientId> = HashMap::new();
    let mut players: HashMap<ClientId, Player> = HashMap::new();
    let level: u8 = 0;

    let mut buf = [0u8; 1400];
    let mut last_tick = Instant::now();

    loop {
        // recv all pending
        while let Ok((n, addr)) = sock.recv_from(&mut buf) {
            if let Ok(msg) = from_bytes::<C2S>(&buf[..n]) {
                match msg {
                    C2S::Hello { name } => {
                        let is_new = !clients.contains_key(&addr);
                        let id = *clients.entry(addr).or_insert_with(|| {
                            let id = next_id;
                            next_id += 1;
                            players.insert(
                                id,
                                Player {
                                    id,
                                    name,
                                    pos: Vec2 { x: 2.5, y: 2.5 },
                                    ang: 0.0,
                                    level,
                                    alive: true,
                                },
                            );
                            id
                        });
                        if is_new {
                            let name_str = String::from_utf8_lossy(&name);
                            let name_str = name_str.trim_end_matches('\0');
                            println!("Client connected: {} (id={}, addr={})", name_str, id, addr);
                        }
                        let pkt = to_stdvec(&S2C::Welcome { id }).unwrap();
                        let _ = sock.send_to(&pkt, addr);
                    }
                    C2S::Input {
                        fwd,
                        strafe,
                        turn,
                        shoot: _,
                    } => {
                        if let Some(&id) = clients.get(&addr) {
                            if let Some(p) = players.get_mut(&id) {
                                // super-simple movement
                                p.ang += turn;
                                let (cs, sn) = (p.ang.cos(), p.ang.sin());
                                let dx = cs * fwd + -sn * strafe;
                                let dy = sn * fwd + cs * strafe;
                                let nx = p.pos.x + dx * 0.08;
                                let ny = p.pos.y + dy * 0.08;
                                if !is_wall(p.level, nx as i32, p.pos.y as i32) {
                                    p.pos.x = nx;
                                }
                                if !is_wall(p.level, p.pos.x as i32, ny as i32) {
                                    p.pos.y = ny;
                                }
                            }
                        }
                    }
                }
            }
        }

        // tick broadcast
        if last_tick.elapsed() >= Duration::from_millis(1000 / TICK_HZ) {
            last_tick = Instant::now();
            let snap = S2C::Snapshot {
                players: players.values().cloned().collect(),
                level,
            };
            let pkt = to_stdvec(&snap).unwrap();
            for (&addr, _) in clients.iter() {
                let _ = sock.send_to(&pkt, addr);
            }
        }

        std::thread::sleep(Duration::from_millis(1));
    }
}
