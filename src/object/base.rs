use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use log::info;
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, RwLockWriteGuard};

use crate::core::session::Session;

use super::player::PlayerGameObject;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GameObjectType {
    None,
    Player
}

impl Default for GameObjectType {
    fn default() -> Self {
        GameObjectType::None
    }
}

#[allow(dead_code)]
#[async_trait]
pub trait GameObjectTrait: Send + Sync {
    async fn update(
        &mut self,
        sessions: &RwLockWriteGuard<HashMap<u32, Arc<RwLock<Session>>>>,
        objects: &RwLockWriteGuard<HashMap<u32, Arc<RwLock<Box<dyn GameObjectTrait>>>>>,
        delta_time: f32,
    ) -> Result<(), String>;    fn get_alive(&self) -> bool;
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

#[allow(unused_variables)]
#[async_trait]
impl GameObjectTrait for GameObject {
    async fn update(
        &mut self,
        sessions: &'life1 tokio::sync::RwLockWriteGuard<
            '_,
            HashMap<u32, Arc<tokio::sync::RwLock<Session>>>,
        >,
        objects: &'life2 tokio::sync::RwLockWriteGuard<
            '_,
            HashMap<u32, Arc<tokio::sync::RwLock<Box<(dyn GameObjectTrait + 'static)>>>>,
        >,
        delta_time: f32,
    ) -> Result<(), String> {
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
