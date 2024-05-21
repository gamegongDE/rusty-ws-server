#[allow(unused_imports)]
use log::{debug, error, info, warn};

use serde::{Deserialize, Serialize};

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
}

#[allow(dead_code)]
impl GameTile {
    pub fn new(pos_x: u32, pos_y: u32, tile_type: GameTileType) -> GameTile {
        GameTile {
            pos_x,
            pos_y,
            tile_type,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum GameTileType {
    Unknown,
    Floor,
    Obstacle,
    Water,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
impl GameMap {
    pub fn new() -> GameMap {
        GameMap { tiles: Vec::new() }
    }

    pub fn get_tile(&self, x: u32, y: u32) -> &GameTile {
        &self.tiles[x as usize][y as usize]
    }

    pub fn set_tile(&mut self, x: u32, y: u32, tile: GameTile) {
        self.tiles[x as usize][y as usize] = tile;
    }
}
