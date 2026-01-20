use serde::{Deserialize, Serialize};

pub const TICK_HZ: u64 = 20;

pub type ClientId = u32;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Player {
    pub id: ClientId,
    pub name: [u8; 16], // fixed-size to avoid heap in packets
    pub pos: Vec2,
    pub ang: f32,
    pub level: u8,
    pub alive: bool,
}

#[derive(Serialize, Deserialize)]
pub enum C2S {
    Hello {
        name: [u8; 16],
    },
    Input {
        fwd: f32,
        strafe: f32,
        turn: f32,
        shoot: bool,
    },
}

#[derive(Serialize, Deserialize)]
pub enum S2C {
    Welcome { id: ClientId },
    Snapshot { players: Vec<Player>, level: u8 },
}

/// 3 levels, increasing dead-ends. (Keep it tiny + hardcoded.)
pub const MAP_W: usize = 16;
pub const MAP_H: usize = 16;

pub const LEVELS: [[[u8; MAP_W]; MAP_H]; 3] = [
    /* level 0 */
    [
        *b"################",
        *b"#......#.......#",
        *b"#.####.#.#####.#",
        *b"#.#....#.....#.#",
        *b"#.#.########.#.#",
        *b"#.#..........#.#",
        *b"#.##########.#.#",
        *b"#......#.....#.#",
        *b"###.##.#.#####.#", // you can tweak these (must be 16 wide)
        *b"#...#...#......#",
        *b"#.###.#######..#",
        *b"#.....#.....#..#",
        *b"#####.#.###.#.##",
        *b"#.....#...#....#",
        *b"#.########.#####",
        *b"################",
    ]
    .map(|row| row.map(|c| if c == b'#' { 1 } else { 0 })),
    /* level 1 */ [[[0u8; MAP_W]; MAP_H]; 16].map(|_| [0; MAP_W]), // replace with real
    /* level 2 */ [[[0u8; MAP_W]; MAP_H]; 16].map(|_| [0; MAP_W]), // replace with real
];

pub fn is_wall(level: u8, x: i32, y: i32) -> bool {
    if x < 0 || y < 0 || x as usize >= MAP_W || y as usize >= MAP_H {
        return true;
    }
    LEVELS[level as usize][y as usize][x as usize] != 0
}
