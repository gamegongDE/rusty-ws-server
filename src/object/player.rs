use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, RwLockWriteGuard};

use crate::core::session::Session;

use super::base::{GameObject, GameObjectTrait};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerGameObject {
    pub base_object: GameObject,
    pub client_id: u32,
    pub facing_x: f32,
    pub facing_y: f32,
    pub move_x: f32,
    pub move_y: f32,
    pub hp: u32,
}

impl PlayerGameObject {
    #[allow(dead_code)]
    pub fn new(client_id: u32) -> Self {
        Self {
            base_object: GameObject::new(super::base::GameObjectType::Player, 0.0, 0.0),
            client_id,
            facing_x: 0.0,
            facing_y: 0.0,
            move_x: 0.0,
            move_y: 0.0,
            hp: 100,
        }
    }
}

#[allow(unused_variables)]
#[async_trait]
impl GameObjectTrait for PlayerGameObject {
    async fn update(
        &mut self,
        sessions: &RwLockWriteGuard<HashMap<u32, Arc<RwLock<Session>>>>,
        objects: &RwLockWriteGuard<HashMap<u32, Arc<RwLock<Box<dyn GameObjectTrait>>>>>,
        delta_time: f32,
    ) -> Result<(), String> {
        

        Ok(())
    }

    fn get_alive(&self) -> bool {
        self.base_object.alive
    }

    fn get_object_type(&self) -> super::base::GameObjectType {
        self.base_object.object_type.clone()
    }

    fn get_object(&mut self) -> &mut GameObject {
        &mut self.base_object
    }

    fn get_object_as_player(&mut self) -> Option<&mut PlayerGameObject> {
        Some(self)
    }
}
