use std::collections::VecDeque;

use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl Difficulty {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "easy" | "1" => Some(Self::Easy),
            "medium" | "2" => Some(Self::Medium),
            "hard" | "3" => Some(Self::Hard),
            _ => None,
        }
    }

    fn settings(self) -> (usize, usize, f32, usize) {
        match self {
            Self::Easy => (31, 23, 0.16, 10),
            Self::Medium => (37, 27, 0.08, 14),
            Self::Hard => (45, 33, 0.025, 18),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TilePos {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Maze {
    pub width: usize,
    pub height: usize,
    /// Row-major: 0 is wall, 1 is walkable.
    pub tiles: Vec<u8>,
    pub spawns: Vec<TilePos>,
    pub objective: TilePos,
}

impl Maze {
    pub fn is_floor(&self, x: isize, y: isize) -> bool {
        x >= 0
            && y >= 0
            && (x as usize) < self.width
            && (y as usize) < self.height
            && self.tiles[y as usize * self.width + x as usize] == 1
    }

    pub fn floor_neighbors(&self, p: TilePos) -> usize {
        [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .into_iter()
            .filter(|(dx, dy)| self.is_floor(p.x as isize + dx, p.y as isize + dy))
            .count()
    }

    pub fn world_position(&self, p: TilePos) -> (f32, f32) {
        let x = (p.x as f32 - self.width as f32 / 2.0 + 0.5) * crate::TILE_SIZE;
        let y = (p.y as f32 - self.height as f32 / 2.0 + 0.5) * crate::TILE_SIZE;
        (x, y)
    }

    pub fn world_is_clear(&self, x: f32, y: f32, radius: f32) -> bool {
        let left = ((x - radius) / crate::TILE_SIZE + self.width as f32 / 2.0).floor() as isize;
        let right = ((x + radius) / crate::TILE_SIZE + self.width as f32 / 2.0).floor() as isize;
        let bottom = ((y - radius) / crate::TILE_SIZE + self.height as f32 / 2.0).floor() as isize;
        let top = ((y + radius) / crate::TILE_SIZE + self.height as f32 / 2.0).floor() as isize;
        self.is_floor(left, bottom)
            && self.is_floor(left, top)
            && self.is_floor(right, bottom)
            && self.is_floor(right, top)
    }
}

pub struct GeneratedMaze {
    pub maze: Maze,
    pub effective_seed: u64,
    pub used_fallback: bool,
}

pub fn generate(seed: u64, difficulty: Difficulty, max_retries: usize) -> GeneratedMaze {
    for attempt in 0..max_retries {
        let derived = seed.wrapping_add((attempt as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let candidate = carve(derived, difficulty);
        if validate(&candidate).is_ok() {
            return GeneratedMaze {
                maze: candidate,
                effective_seed: derived,
                used_fallback: false,
            };
        }
    }
    // This fixed seed is generated through the same algorithm and is part of the
    // compatibility contract, making fallback topology reproducible.
    let fallback_seed = 0x4D41_5A45_5741_5253;
    let maze = carve(fallback_seed, Difficulty::Easy);
    assert!(
        validate(&maze).is_ok(),
        "built-in fallback maze must remain valid"
    );
    GeneratedMaze {
        maze,
        effective_seed: fallback_seed,
        used_fallback: true,
    }
}

pub fn empty(seed: u64, difficulty: Difficulty) -> GeneratedMaze {
    let (width, height, _, _) = difficulty.settings();
    let spawns = [
        TilePos { x: 2, y: 2 },
        TilePos { x: width / 2, y: 2 },
        TilePos { x: width - 3, y: 2 },
        TilePos {
            x: 2,
            y: height / 2,
        },
        TilePos {
            x: width - 3,
            y: height / 2,
        },
        TilePos {
            x: 2,
            y: height - 3,
        },
        TilePos {
            x: width / 2,
            y: height - 3,
        },
        TilePos {
            x: width - 3,
            y: height - 3,
        },
        TilePos {
            x: width / 3,
            y: height / 2,
        },
        TilePos {
            x: width * 2 / 3,
            y: height / 2,
        },
    ]
    .into();

    GeneratedMaze {
        maze: Maze {
            width,
            height,
            tiles: vec![1; width * height],
            spawns,
            objective: TilePos {
                x: width / 2,
                y: height / 2,
            },
        },
        effective_seed: seed,
        used_fallback: false,
    }
}

fn carve(seed: u64, difficulty: Difficulty) -> Maze {
    let (width, height, loop_chance, min_spawn_distance) = difficulty.settings();
    let mut rng = StdRng::seed_from_u64(seed);
    let mut tiles = vec![0; width * height];
    let mut stack = vec![(1usize, 1usize)];
    tiles[width + 1] = 1;
    while let Some(&(x, y)) = stack.last() {
        let mut directions = [(2isize, 0isize), (-2, 0), (0, 2), (0, -2)];
        directions.shuffle(&mut rng);
        let next = directions.into_iter().find_map(|(dx, dy)| {
            let nx = x as isize + dx;
            let ny = y as isize + dy;
            (nx > 0
                && ny > 0
                && nx < width as isize - 1
                && ny < height as isize - 1
                && tiles[ny as usize * width + nx as usize] == 0)
                .then_some((nx as usize, ny as usize, dx, dy))
        });
        if let Some((nx, ny, dx, dy)) = next {
            tiles[(y as isize + dy / 2) as usize * width + (x as isize + dx / 2) as usize] = 1;
            tiles[ny * width + nx] = 1;
            stack.push((nx, ny));
        } else {
            stack.pop();
        }
    }
    // Loops create alternate exits from corridors. Easy maps intentionally have
    // more of them, while hard maps retain the DFS maze's longer routes/dead ends.
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            if tiles[y * width + x] == 0 && rng.gen::<f32>() < loop_chance {
                let horizontal = tiles[y * width + x - 1] == 1 && tiles[y * width + x + 1] == 1;
                let vertical = tiles[(y - 1) * width + x] == 1 && tiles[(y + 1) * width + x] == 1;
                if horizontal ^ vertical {
                    tiles[y * width + x] = 1;
                }
            }
        }
    }
    let mut maze = Maze {
        width,
        height,
        tiles,
        spawns: Vec::new(),
        objective: TilePos { x: 1, y: 1 },
    };
    let distances = distances_from(&maze, TilePos { x: 1, y: 1 });
    maze.objective = farthest(&maze, &distances);

    let mut candidates: Vec<_> = (1..height - 1)
        .flat_map(|y| (1..width - 1).map(move |x| TilePos { x, y }))
        .filter(|&p| maze.is_floor(p.x as isize, p.y as isize) && maze.floor_neighbors(p) >= 2)
        .collect();
    candidates.shuffle(&mut rng);
    for p in candidates {
        if maze.spawns.len() >= 16 {
            break;
        }
        let acceptable = maze.spawns.iter().all(|&other| {
            path_distance(&maze, p, other).is_some_and(|d| d >= min_spawn_distance)
                && !clear_firing_line(&maze, p, other)
        });
        if acceptable {
            maze.spawns.push(p);
        }
    }
    maze
}

pub fn validate(maze: &Maze) -> Result<(), String> {
    if maze.spawns.len() < 10 {
        return Err("fewer than ten safe spawns".into());
    }
    if !maze.is_floor(maze.objective.x as isize, maze.objective.y as isize) {
        return Err("objective is blocked".into());
    }
    let start = maze.spawns[0];
    let distances = distances_from(maze, start);
    let floor_count = maze.tiles.iter().filter(|&&t| t == 1).count();
    if distances.iter().filter(|&&d| d != usize::MAX).count() != floor_count {
        return Err("walkable region is disconnected".into());
    }
    if maze
        .spawns
        .iter()
        .any(|&p| maze.floor_neighbors(p) < 2 || distances[p.y * maze.width + p.x] == usize::MAX)
    {
        return Err("unsafe or unreachable spawn".into());
    }
    if distances[maze.objective.y * maze.width + maze.objective.x] == usize::MAX {
        return Err("objective is unreachable".into());
    }
    Ok(())
}

fn distances_from(maze: &Maze, start: TilePos) -> Vec<usize> {
    let mut distances = vec![usize::MAX; maze.width * maze.height];
    let mut queue = VecDeque::from([start]);
    distances[start.y * maze.width + start.x] = 0;
    while let Some(p) = queue.pop_front() {
        let distance = distances[p.y * maze.width + p.x];
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let nx = p.x as isize + dx;
            let ny = p.y as isize + dy;
            if maze.is_floor(nx, ny) {
                let index = ny as usize * maze.width + nx as usize;
                if distances[index] == usize::MAX {
                    distances[index] = distance + 1;
                    queue.push_back(TilePos {
                        x: nx as usize,
                        y: ny as usize,
                    });
                }
            }
        }
    }
    distances
}

fn farthest(maze: &Maze, distances: &[usize]) -> TilePos {
    let (index, _) = distances
        .iter()
        .enumerate()
        .filter(|(_, d)| **d != usize::MAX)
        .max_by_key(|(_, d)| **d)
        .unwrap();
    TilePos {
        x: index % maze.width,
        y: index / maze.width,
    }
}

fn path_distance(maze: &Maze, a: TilePos, b: TilePos) -> Option<usize> {
    let distance = distances_from(maze, a)[b.y * maze.width + b.x];
    (distance != usize::MAX).then_some(distance)
}

fn clear_firing_line(maze: &Maze, a: TilePos, b: TilePos) -> bool {
    if a.x == b.x {
        let (start, end) = if a.y < b.y { (a.y, b.y) } else { (b.y, a.y) };
        (start + 1..end).all(|y| maze.is_floor(a.x as isize, y as isize))
    } else if a.y == b.y {
        let (start, end) = if a.x < b.x { (a.x, b.x) } else { (b.x, a.x) };
        (start + 1..end).all(|x| maze.is_floor(x as isize, a.y as isize))
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_maps_satisfy_invariants_across_difficulties_and_seeds() {
        for difficulty in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard] {
            for seed in 0..20 {
                let generated = generate(seed, difficulty, 8);
                validate(&generated.maze).unwrap();
                assert!(generated.maze.spawns.len() >= 10);
            }
        }
    }

    #[test]
    fn zero_retries_uses_known_valid_fallback() {
        let generated = generate(123, Difficulty::Hard, 0);
        assert!(generated.used_fallback);
        validate(&generated.maze).unwrap();
    }
}
