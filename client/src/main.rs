use macroquad::prelude::*;
use maze_wars::common::*;
use postcard::{from_bytes, to_stdvec};
use std::{
    io::{self, Write},
    net::UdpSocket,
    time::Instant,
};

fn read_line(prompt: &str) -> String {
    print!("{prompt}");
    let _ = io::stdout().flush();
    let mut s = String::new();
    io::stdin().read_line(&mut s).unwrap();
    s.trim().to_string()
}

fn to_name16(s: &str) -> [u8; 16] {
    let mut out = [0u8; 16];
    let b = s.as_bytes();
    let n = b.len().min(16);
    out[..n].copy_from_slice(&b[..n]);
    out
}

#[macroquad::main("Maze Wars")]
async fn main() {
    let server = read_line("Enter IP Address: ");
    let name = to_name16(&read_line("Enter Name: "));
    println!("Starting...");

    let sock = UdpSocket::bind("0.0.0.0:0").unwrap();
    sock.set_nonblocking(true).unwrap();
    sock.connect(server).unwrap();

    sock.send(&to_stdvec(&C2S::Hello { name }).unwrap())
        .unwrap();

    let mut my_id: ClientId = 0;
    let mut players: Vec<Player> = vec![];
    let mut level: u8 = 0;

    let mut buf = [0u8; 1400];
    let mut last_send = Instant::now();

    loop {
        // --- receive packets (nonblocking)
        while let Ok(n) = sock.recv(&mut buf) {
            if let Ok(msg) = from_bytes::<S2C>(&buf[..n]) {
                match msg {
                    S2C::Welcome { id } => my_id = id,
                    S2C::Snapshot {
                        players: ps,
                        level: lv,
                    } => {
                        players = ps;
                        level = lv;
                    }
                }
            }
        }

        // --- send input (30Hz is enough)
        if last_send.elapsed().as_millis() >= 33 {
            last_send = Instant::now();
            let fwd = (is_key_down(KeyCode::W) as i32 - is_key_down(KeyCode::S) as i32) as f32;
            let strafe = (is_key_down(KeyCode::D) as i32 - is_key_down(KeyCode::A) as i32) as f32;
            let turn = (is_key_down(KeyCode::Right) as i32 - is_key_down(KeyCode::Left) as i32)
                as f32
                * 0.06;
            let shoot = is_key_pressed(KeyCode::Space);
            let pkt = to_stdvec(&C2S::Input {
                fwd,
                strafe,
                turn,
                shoot,
            })
            .unwrap();
            let _ = sock.send(&pkt);
        }

        // --- render
        clear_background(BLACK);

        if let Some(me) = players.iter().find(|p| p.id == my_id) {
            draw_3d_view(me, &players, level);
            draw_minimap(me, &players, level);
        }

        draw_text(&format!("FPS: {}", get_fps()), 10.0, 20.0, 24.0, GREEN);

        next_frame().await;
    }
}

fn draw_3d_view(me: &Player, players: &[Player], level: u8) {
    let w = screen_width();
    let h = screen_height();
    let fov = 1.1; // ~63 deg
    let rays = w as i32;

    for i in 0..rays {
        let t = (i as f32 / (rays as f32 - 1.0)) * 2.0 - 1.0;
        let ang = me.ang + t * (fov * 0.5);
        let dist = raycast(level, me.pos, ang);
        let wall_h = (h / (dist.max(0.001))) * 0.08;
        let x = i as f32;
        let y = (h - wall_h) * 0.5;
        draw_rectangle(x, y, 1.0, wall_h, DARKGRAY);
    }

    // ultra-minimal “other players”: project as a vertical stripe if roughly in front
    for p in players.iter().filter(|p| p.id != me.id) {
        let dx = p.pos.x - me.pos.x;
        let dy = p.pos.y - me.pos.y;
        let dist = (dx * dx + dy * dy).sqrt().max(0.001);
        let ang_to = dy.atan2(dx);
        let rel = wrap_angle(ang_to - me.ang);
        if rel.abs() < fov * 0.5 {
            let sx = ((rel / (fov * 0.5)) * 0.5 + 0.5) * w;
            let ph = (h / dist) * 0.05;
            draw_rectangle(sx - 2.0, (h - ph) * 0.5, 4.0, ph, RED);
        }
    }
}

fn raycast(level: u8, pos: Vec2, ang: f32) -> f32 {
    // DDA-ish “cheap” stepping: minimal code, good enough for Maze Wars visuals
    let (cs, sn) = (ang.cos(), ang.sin());
    let mut t = 0.0;
    loop {
        t += 0.03;
        let x = pos.x + cs * t;
        let y = pos.y + sn * t;
        if is_wall(level, x as i32, y as i32) || t > 30.0 {
            return t;
        }
    }
}

fn draw_minimap(me: &Player, players: &[Player], level: u8) {
    let scale = 8.0;
    let ox = 10.0;
    let oy = 40.0;

    // world
    for y in 0..MAP_H {
        for x in 0..MAP_W {
            if LEVELS[level as usize][y][x] != 0 {
                draw_rectangle(
                    ox + x as f32 * scale,
                    oy + y as f32 * scale,
                    scale,
                    scale,
                    GRAY,
                );
            }
        }
    }

    // players
    for p in players {
        let px = ox + p.pos.x * scale;
        let py = oy + p.pos.y * scale;
        draw_circle(px, py, 2.5, if p.id == me.id { GREEN } else { YELLOW });
    }
}

fn wrap_angle(mut a: f32) -> f32 {
    while a > std::f32::consts::PI {
        a -= 2.0 * std::f32::consts::PI;
    }
    while a < -std::f32::consts::PI {
        a += 2.0 * std::f32::consts::PI;
    }
    a
}
