use async_trait::async_trait;
use log::info;
use serde::{Deserialize, Serialize};

use crate::core::server::{GameObjects, GameObjectsArc, Sessions, SessionsArc};

use super::player::PlayerGameObject;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GameObjectType {
    None,
    Player,
    AI,
    Bullet,
    Item,
}

impl Default for GameObjectType {
    fn default() -> Self {
        GameObjectType::None
    }
}

#[async_trait]
pub trait GameObjectTrait: Send + Sync {
    //fn update(&mut self, clients: SharedClients, state: SharedState) -> Result<(), String>;
    async fn update(&mut self, sessions: &mut Sessions, objects: &tokio::sync::RwLockWriteGuard<'_, GameObjects>, delta_time: f32) -> Result<(), String>;
    fn get_alive(&self) -> bool;
    fn get_object_type(&self) -> GameObjectType;
    fn get_object(&mut self) -> &mut GameObject;
    fn get_object_as_player(&mut self) -> Option<&mut PlayerGameObject>;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameObject {
    pub alive: bool,
    pub object_type: GameObjectType,
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
}

impl Default for GameObject {
    fn default() -> Self {
        GameObject {
            alive: true,
            object_type: GameObjectType::default(),
            pos_x: 0.0,
            pos_y: 0.0,
            pos_z: 0.0,
        }
    }
}

impl GameObject {
    pub fn new(object_type: GameObjectType, pos_x: f32, pos_y: f32) -> Self {
        let mut new_gameobj = GameObject::default();
        new_gameobj.object_type = object_type;
        new_gameobj.pos_x = pos_x;
        new_gameobj.pos_y = pos_y;

        new_gameobj
    }
}

#[async_trait]
impl GameObjectTrait for GameObject {
    async fn update(&mut self, _sessions: &mut Sessions, _objects: &tokio::sync::RwLockWriteGuard<'_, GameObjects>, _delta_time: f32) -> Result<(), String> {
        // update game object
        info!("GameObject::update() called");
        Ok(())
    }

    fn get_alive(&self) -> bool {
        self.alive
    }

    fn get_object_type(&self) -> GameObjectType {
        self.object_type.clone()
    }

    fn get_object(&mut self) -> &mut GameObject {
        self
    }

    fn get_object_as_player(&mut self) -> Option<&mut PlayerGameObject> {
        None
    }
}
