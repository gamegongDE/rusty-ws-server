use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fs::File;
use std::io::Read;

#[derive(Serialize, Deserialize, Debug)]
struct RawJsonMap {
    column_count: u32,
    row_count: u32,
    tiles: Vec<RawJsonTile>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RawJsonTile {
    #[serde(rename = "type")]
    tile_type: u32,
    texture_id: u32,
}

#[derive(Debug)]
pub struct GameMap {
    pub tiles: Vec<Vec<GameTile>>,
}

#[derive(Debug)]
pub struct GameTile {
    pub tile_type: GameTileType,
    pub pos_x: u32,
    pub pos_y: u32,
    pub pos_z: u32,       // 필요할까?
    pub render_type: u32, // 필요할까?
}

impl GameTile {
    pub fn new(
        pos_x: u32,
        pos_y: u32,
        pos_z: u32,
        tile_type: GameTileType,
        render_type: u32,
    ) -> GameTile {
        GameTile {
            pos_x,
            pos_y,
            pos_z,
            tile_type,
            render_type,
        }
    }
}

#[derive(Debug)]
pub enum GameTileType {
    Unknown,
    Floor,
    Obstacle,
    Water,
}

impl GameTileType {
    pub fn get_id(&self) -> u32 {
        match self {
            GameTileType::Floor => 0,
            GameTileType::Obstacle => 1,
            GameTileType::Water => 2,
            _ => panic!("Unknown tile type"),
        }
    }
    /// 부피가 있는 타일인지 확인한다.
    pub fn has_volume(&self) -> bool {
        match self {
            GameTileType::Floor => false,
            GameTileType::Obstacle => true,
            GameTileType::Water => false,
            _ => panic!("Unknown tile type"),
        }
    }

    pub fn is_steppable(&self) -> bool {
        match self {
            GameTileType::Floor => true,
            GameTileType::Obstacle => false,
            GameTileType::Water => false,
            _ => panic!("Unknown tile type"),
        }
    }
}

impl GameMap {
    pub fn new() -> GameMap {
        let map_file = File::open("src/assets/map.json");
        let mut contents = String::new();

        // temp
        return GameMap {
            tiles: Vec::new()
        };

        if map_file.is_err() {
            panic!("error occured while opening map file.")
        }

        let _ = map_file.unwrap().read_to_string(&mut contents);

        let to_be_game_map = serde_json::from_str(&contents);

        if to_be_game_map.is_err() {
            panic!("error occured while parsing map file.")
        }

        let raw_json_map: RawJsonMap = to_be_game_map.unwrap();

        let mut tiles: Vec<Vec<GameTile>> = Vec::new();

        for y in 0..raw_json_map.row_count {
            let mut row: Vec<GameTile> = Vec::new();
            for x in 0..raw_json_map.column_count {
                let raw_tile = &raw_json_map.tiles[(y * raw_json_map.column_count + x) as usize];
                let tile_type = match raw_tile.tile_type {
                    0 => GameTileType::Floor,
                    1 => GameTileType::Obstacle,
                    2 => GameTileType::Water,
                    _ => GameTileType::Unknown,
                };
                row.push(GameTile::new(x, y, 0, tile_type, 0));
            }
            tiles.push(row);
        }

        GameMap { tiles }
    }

    pub fn get_tile(&self, x: u32, y: u32) -> &GameTile {
        &self.tiles[x as usize][y as usize]
    }

    pub fn set_tile(&mut self, x: u32, y: u32, tile: GameTile) {
        self.tiles[x as usize][y as usize] = tile;
    }

    pub fn raycast(&self, start_x: u32, start_y: u32, end_x: u32, end_y: u32) -> bool {
        let mut x = start_x as f32;
        let mut y = start_y as f32;
        let dx = (end_x as f32 - start_x as f32).abs();
        let dy = (end_y as f32 - start_y as f32).abs();
        let sx = if start_x < end_x { 1 } else { -1 };
        let sy = if start_y < end_y { 1 } else { -1 };
        let mut err = dx - dy;

        loop {
            if x == end_x as f32 && y == end_y as f32 {
                break;
            }
            let e2 = 2.0 * err as f32;
            if e2 > -dy {
                err -= dy;
                x += sx as f32;
            }
            if e2 < dx {
                err += dx;
                y += sy as f32;
            }
            if self.get_tile(x as u32, y as u32).tile_type.has_volume() {
                return false;
            }
        }
        true
    }

    pub fn get_path(&self, start_x: u32, start_y: u32, end_x: u32, end_y: u32) -> Vec<(u32, u32)> {
        // Initialize the open and closed sets for A* algorithm.
        let mut open_set: BinaryHeap<AStarNode> = BinaryHeap::new();
        let mut came_from: HashMap<(u32, u32), (u32, u32)> = HashMap::new();

        // G and F scores maps.
        let mut g_score: HashMap<(u32, u32), u32> = HashMap::new();
        let mut f_score: HashMap<(u32, u32), u32> = HashMap::new();

        // Initialize g_score and f_score for the start position.
        g_score.insert((start_x, start_y), 0);
        f_score.insert(
            (start_x, start_y),
            self.heuristic(start_x, start_y, end_x, end_y),
        );

        // Add the start node to the open set.
        open_set.push(AStarNode {
            x: start_x,
            y: start_y,
            f: self.heuristic(start_x, start_y, end_x, end_y),
            g: 0,
            h: self.heuristic(start_x, start_y, end_x, end_y),
        });

        while let Some(current) = open_set.pop() {
            // Check if the current node is the goal.
            if current.x == end_x && current.y == end_y {
                return self.reconstruct_path(came_from, current.x, current.y);
            }

            // Generate neighbors (assuming 4-directional movement).
            let neighbors = self.get_neighbors(current.x, current.y);

            for (nx, ny) in neighbors {
                // Skip if not movable.
                if !self.tiles[nx as usize][ny as usize]
                    .tile_type
                    .is_steppable()
                {
                    continue;
                }

                // Tentative gScore is the gScore of the current node plus the distance between current and neighbor.
                let tentative_g_score = current.g + 1; // Assuming cost between adjacent nodes is 1.

                if tentative_g_score < *g_score.get(&(nx, ny)).unwrap_or(&u32::MAX) {
                    // This path to neighbor is better than any previous one. Record it!
                    came_from.insert((nx, ny), (current.x, current.y));
                    g_score.insert((nx, ny), tentative_g_score);
                    f_score.insert(
                        (nx, ny),
                        tentative_g_score + self.heuristic(nx, ny, end_x, end_y),
                    );

                    if open_set.iter().all(|&node| node.x != nx || node.y != ny) {
                        open_set.push(AStarNode {
                            x: nx,
                            y: ny,
                            f: tentative_g_score + self.heuristic(nx, ny, end_x, end_y),
                            g: tentative_g_score,
                            h: self.heuristic(nx, ny, end_x, end_y),
                        });
                    }
                }
            }
        }

        // If the goal was never reached, return an empty path.
        Vec::new()
    }

    // Heuristic function for A* (Manhattan distance in this case).
    fn heuristic(&self, x1: u32, y1: u32, x2: u32, y2: u32) -> u32 {
        ((x1 as i32 - x2 as i32).abs() + (y1 as i32 - y2 as i32).abs()) as u32
    }

    // Reconstructs the path from the came_from map.
    fn reconstruct_path(
        &self,
        came_from: HashMap<(u32, u32), (u32, u32)>,
        current_x: u32,
        current_y: u32,
    ) -> Vec<(u32, u32)> {
        let mut path = vec![(current_x, current_y)];
        let mut current = (current_x, current_y);
        while let Some(&next) = came_from.get(&current) {
            path.push(next);
            current = next;
        }
        path.reverse(); // Reverse the path to start from the beginning.
        path
    }

    // Get the neighbors of a given tile (assuming 4-directional movement).
    fn get_neighbors(&self, x: u32, y: u32) -> Vec<(u32, u32)> {
        let mut neighbors = Vec::new();
        if x > 0 {
            neighbors.push((x - 1, y));
        }
        if y > 0 {
            neighbors.push((x, y - 1));
        }
        if x < self.tiles.len() as u32 - 1 {
            neighbors.push((x + 1, y));
        }
        if y < self.tiles[0].len() as u32 - 1 {
            neighbors.push((x, y + 1));
        }
        neighbors
    }
}

// A structure to hold a node in the pathfinding process, including its x and y coordinates,
// and f, g, h scores for A* algorithm.
#[derive(Copy, Clone, Eq, PartialEq)]
struct AStarNode {
    x: u32,
    y: u32,
    f: u32,
    g: u32,
    h: u32,
}

// Implement ordering for AStarNode to be used in the priority queue (BinaryHeap).
impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Note that we flip other and self here to turn the min-heap into a max-heap.
        other.f.cmp(&self.f).then_with(|| other.h.cmp(&self.h))
    }
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
