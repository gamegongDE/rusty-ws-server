use std::collections::HashMap;

use crate::object::base::GameObjectTrait;
use crate::{
    handler::handler_echo::handle_echo,
    msg::msg_in::MsgIn,
    server::{ClientMap, SharedClients, SharedState},
};
use log::{debug, error, info};
use tokio::select;
use tokio::sync::watch;
use warp::filters::ws::Message;

pub struct UpdateThread {
    pub clients: SharedClients,
    pub state: SharedState,
}

impl UpdateThread {
    pub fn new(clients: SharedClients, state: SharedState) -> UpdateThread {
        UpdateThread { clients, state }
    }

    pub async fn run(
        &mut self,
        mut ternimate_receiver: watch::Receiver<bool>,
    ) -> Result<(), String> {
        info!("Logic thread started");

        let mut last_update = tokio::time::Instant::now();
        let mut last_fps = 0;
        let mut last_fps_checked = tokio::time::Instant::now();
        let mut delta_time;
        loop {
            let now = tokio::time::Instant::now();
            delta_time = now.duration_since(last_update).as_secs_f32();
            self.update(delta_time).await;
            last_update = tokio::time::Instant::now();
            last_fps += 1;
            if last_fps_checked.elapsed().as_millis() >= 1000 {
                last_fps_checked = tokio::time::Instant::now();
                debug!("FPS: {}", last_fps);
                last_fps = 0;
            }
            select! {
                _ = tokio::time::sleep_until(last_update + tokio::time::Duration::from_millis(30)) => {

                }
                _ = ternimate_receiver.changed() => {
                    if *ternimate_receiver.borrow() {
                        info!("Terminate signal received");
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn process_message(
        &self,
        message: MsgIn,
        client_key: u32,
        mut clients_writelock: &mut tokio::sync::RwLockWriteGuard<'_, ClientMap>,
        mut state_writelock: &mut tokio::sync::RwLockWriteGuard<'_, crate::server::GameState>,
    ) {
        match message {
            MsgIn::Echo(data) => {
                let handle_result = handle_echo(
                    &data,
                    client_key,
                    &mut clients_writelock,
                    &mut state_writelock,
                )
                .await;
                match handle_result {
                    Ok(()) => {}
                    Err(e) => {
                        error!("Error handling message: {}", e);
                    }
                }
            }
        }
    }

    pub async fn update(&self, delta_time: f32) {
        // info!("update");

        let mut clients_writelock = self.clients.write().await;
        let mut state_writelock = self.state.write().await;

        // 1단계: 메시지 수집
        let mut messages_to_process: HashMap<u32, Vec<MsgIn>> = HashMap::new();

        // 클라이언트를 순회하며 살아있는 클라이언트의 메시지 처리
        for (client_key, client) in clients_writelock.iter_mut() {
            if client.get_alive() {
                while let Some(message) = client.message_inbound.pop_front() {
                    messages_to_process
                        .entry(*client_key)
                        .or_insert(Vec::new())
                        .push(message);
                }
            }
        }

        // 1-1단계: 메시지 처리
        for (client_key, messages) in messages_to_process.iter() {
            for message in messages.iter() {
                self.process_message(
                    message.clone(),
                    *client_key,
                    &mut clients_writelock,
                    &mut state_writelock,
                )
                .await;
            }
        }

        // 2단계: 게임 로직 처리
        // state_writelock 의 player_objects 를 순회하면서 update 함수를 호출한다
        for (_key, player_object) in state_writelock.player_objects.iter() {
            let mut write_lock = player_object.write().await;

            let _result = write_lock
                .update(&mut clients_writelock, &state_writelock, delta_time).await;
        }
        // state_writelock 의 game_objects 를 순회하면서 update 함수를 호출한다
        for (_key, game_object) in state_writelock.game_objects.iter() {
            let mut write_lock = game_object.write().await;
            let _result = write_lock
                .update(&mut clients_writelock, &state_writelock, delta_time).await;
        }

        // 3단계: 메시지 전송
        for (_, client) in clients_writelock.iter_mut() {
            if client.get_alive() == false {
                continue;
            }
            for message in client.message_outbound.drain(..) {
                let message_json =
                    serde_json::to_string(&message).expect("Failed to serialize message");
                client
                    .channel
                    .send(Ok(Message::text(message_json)))
                    .expect("Failed to send message");
            }
        }

        drop(state_writelock);
        drop(clients_writelock);
    }
}
