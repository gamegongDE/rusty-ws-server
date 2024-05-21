#[allow(unused_imports)]
use log::{debug, error, info, warn};
use tungstenite::protocol;

use crate::core::session::Session;
use crate::handler::handler_echo::handle_echo;
use crate::msg::msg_in::MsgIn;
use crate::object::base::GameObjectTrait;
use crate::system::map::GameMap;
use futures::{join, FutureExt, SinkExt, StreamExt};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::atomic::AtomicU32;
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
use tokio::signal;
#[allow(unused_imports)]
#[cfg(target_os = "windows")]
use tokio::signal::windows::ctrl_c;
use tokio::sync::{watch, RwLock};
use warp::ws::{Message, WebSocket};
use warp::Filter;

pub type Sessions = HashMap<u32, Arc<RwLock<Session>>>;
pub type SessionsArc = Arc<RwLock<Sessions>>;
pub type GameObjects = HashMap<u32, Arc<RwLock<Box<dyn GameObjectTrait>>>>;
pub type GameObjectsArc = Arc<RwLock<GameObjects>>;
pub type GameMapArc = Arc<RwLock<GameMap>>;

#[allow(dead_code)]
#[derive(Clone)]
pub(crate) struct GameServer {
    listen_address: SocketAddr,
    timestamp_server_start: u64,
    timestamp_server_end: u64,
    sessions: SessionsArc,

    game_objects: GameObjectsArc,
    game_map: GameMapArc,

    next_session_key: Arc<AtomicU32>,
    next_object_key: u32,
}

impl GameServer {
    pub async fn new(addr: SocketAddr) -> Result<Self, std::io::Error> {
        let listen_address = addr;
        let sessions = Arc::new(RwLock::new(HashMap::new()));
        let game_objects = Arc::new(RwLock::new(HashMap::new()));
        let game_map = Arc::new(RwLock::new(GameMap::new()));
        let next_session_key = Arc::new(AtomicU32::new(1000));
        let next_object_key = 1000000;
        Ok(Self {
            listen_address,
            timestamp_server_start: 0,
            timestamp_server_end: 0,
            sessions,
            game_objects,
            game_map,
            next_session_key,
            next_object_key,
        })
    }

    pub async fn run(&mut self) -> Result<(), String> {
        let total_thread_cnt = num_cpus::get();
        let logic_thread_cnt = 1;
        let network_task_cnt = {
            let calculated_cnt = total_thread_cnt - 4;
            let adjusted_cnt = calculated_cnt - (calculated_cnt % 4);
            if adjusted_cnt < 1 {
                1
            } else if adjusted_cnt > 16 {
                16
            } else {
                adjusted_cnt
            }
        };

        info!(
            "Game WebSocket server listening on: {}",
            self.listen_address
        );
        info!("Total thread count: {}", total_thread_cnt);
        info!("Logic thread count: {}", logic_thread_cnt);
        info!("Network task count: {}", network_task_cnt);

        let (terminate_sender, terminate_receiver) = watch::channel(false);
        // shutdown_signal 태스크를 생성합니다.
        let _ = tokio::spawn(async {
            Self::shutdown_signal(terminate_sender).await;
        });

        // network task 를 생성합니다.
        let mut fps_receivers = Vec::new();

        for i in 0..network_task_cnt {
            let copied_terminate_receiver = terminate_receiver.clone();
            let copied_sessions = self.sessions.clone();
            let worker_index = i;
            let worker_count = network_task_cnt;
            let (fps_sender_task, fps_receiver_task) = watch::channel(0);
            fps_receivers.push(fps_receiver_task);
            let _ = tokio::spawn(async move {
                Self::process_network(
                    copied_sessions,
                    worker_index,
                    worker_count,
                    copied_terminate_receiver,
                    fps_sender_task,
                )
                .await;
            });
        }

        let sessions_clone = self.sessions.clone();
        let session_warp = warp::any().map(move || sessions_clone.clone());
        let session_key_clone = self.next_session_key.clone();
        let session_key_warp = warp::any().map(move || session_key_clone.clone());

        let command_handler = warp::path("command").map(|| "de game command handler");

        let intro = warp::path::end().map(|| "de game server");

        // GET /ws
        let ws = warp::path("ws")
            .and(warp::ws())
            .and(warp::addr::remote())
            .and(session_warp)
            .and(session_key_warp)
            .map(
                |ws: warp::ws::Ws, addr: Option<SocketAddr>, sessions, next_session_key| {
                    ws.on_upgrade(move |socket| {
                        Self::handle_connection(socket, addr, sessions, next_session_key)
                    })
                },
            );

        let routes = ws.or(command_handler).or(intro);

        let mut copied_terminate_receiver = terminate_receiver.clone();
        let (_, server) =
            warp::serve(routes).bind_with_graceful_shutdown(self.listen_address, async move {
                copied_terminate_receiver.changed().await.unwrap();
            });

        self.timestamp_server_start = chrono::Utc::now().timestamp_millis() as u64;
        warn!("Server started at {}", self.timestamp_server_start);

        // 웹소켓 서버를 시작합니다.
        let tokio_server_handler = tokio::spawn(server);

        // 로직 루프를 실행합니다.
        let mut last_update = tokio::time::Instant::now();
        let mut last_fps = 0;
        let mut last_fps_checked = tokio::time::Instant::now();
        let target_fps = 60.0;
        let mut next_target_frame_time =
            tokio::time::Instant::now() + Duration::from_secs_f32(1.0 / target_fps);

        let copied_terminate_receiver = terminate_receiver.clone();
        loop {
            let now = tokio::time::Instant::now();
            let delta_time = now.duration_since(last_update).as_secs_f32();
            last_update = now;

            self.process_update(delta_time).await;

            last_fps += 1;

            // 평균 FPS 계산
            let mut total_fps = 0;
            for fps_receiver in &fps_receivers {
                total_fps += *fps_receiver.borrow();
            }
            let last_network_average_fps = total_fps / network_task_cnt;

            if last_fps_checked.elapsed().as_millis() >= 1000 {
                last_fps_checked = now;
                debug!(
                    "FPS: (logic) {} (network) {}",
                    last_fps, last_network_average_fps
                );
                last_fps = 0;
            }

            if *copied_terminate_receiver.borrow() {
                warn!("Terminate signal received");
                break;
            }

            let sleep_time = next_target_frame_time.duration_since(now);
            if sleep_time.as_secs_f32() > 0.0 {
                tokio::time::sleep(sleep_time).await;
            }
            next_target_frame_time =
                next_target_frame_time + Duration::from_secs_f32(1.0 / target_fps);
        }

        let _ = join!(tokio_server_handler);

        self.timestamp_server_end = chrono::Utc::now().timestamp_millis() as u64;
        warn!("Server finished at {}", self.timestamp_server_end);

        Ok(())
    }

    /**
     * handle new websocket connection
     */
    #[allow(unused_variables)]
    pub async fn handle_connection(
        ws: WebSocket,
        addr: Option<SocketAddr>,
        sessions: SessionsArc,
        session_key: Arc<AtomicU32>,
    ) {
        let session_key = session_key.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        debug!(
            "Session #{} connected (from {})",
            session_key,
            addr.unwrap()
        );

        let (user_tx, user_rx) = ws.split();

        let session = Session {
            alive: true,
            key: session_key,
            address: addr,
            session: "".to_string(),
            channel_send: user_tx,
            channel_recv: user_rx,
            message_inbound: VecDeque::new(),
            message_outbound: VecDeque::new(),
            user_index: None,        // unauthorized now
            user_access_token: None, // unauthorized now
            player_object_key: None, // no player object now
        };

        sessions
            .write()
            .await
            .insert(session_key, Arc::new(RwLock::new(session)));
    }

    pub async fn process_network(
        sessions: SessionsArc,
        worker_index: usize,
        worker_count: usize,
        terminate_receiver: watch::Receiver<bool>,
        fps_sender: watch::Sender<usize>,
    ) {
        let mut last_fps_checked = tokio::time::Instant::now();
        let mut frame_count = 0;

        loop {
            if *terminate_receiver.borrow() {
                break;
            }
            let sessions_readlock = sessions.read().await;

            for (session_key, session) in sessions_readlock.iter() {
                if session_key % worker_count as u32 != worker_index as u32 {
                    continue;
                }
                let mut session = session.write().await;

                loop {
                    let recv_msg = session.channel_recv.next().now_or_never();
                    if recv_msg.is_none() {
                        break;
                    }
                    let recv_msg = recv_msg.unwrap();
                    if recv_msg.is_none() {
                        break;
                    }
                    let recv_msg = recv_msg.unwrap();
                    match recv_msg {
                        Ok(message) => {
                            if message.is_close() {
                                session.alive = false;
                                break;
                            }
                            if !message.is_text() {
                                return;
                            }

                            let message_parse: Result<MsgIn, _> =
                                serde_json::from_str(message.to_str().unwrap());
                            if message_parse.is_err() {
                                warn!("Message parse error : {}", message.to_str().unwrap());
                                return;
                            }
                            let message = message_parse.unwrap();
                            debug!("Message: {:?}", message);

                            session.message_inbound.push_back(message);
                        }
                        Err(err) => {
                            warn!("Error receiving message: {}", err);
                            session.alive = false;
                            break;
                        }
                    }
                }

                let mut sending_messages = vec![];
                for message in session.message_outbound.drain(..) {
                    let message_json = serde_json::to_string(&message);
                    if message_json.is_err() {
                        warn!("Failed to serialize message: {:?}", message);
                        continue;
                    }
                    sending_messages.push(message_json.unwrap());
                }

                for message in sending_messages.iter() {
                    let _ = session.channel_send.send(Message::text(message)).await;
                }
            }

            drop(sessions_readlock);

            frame_count += 1;

            let now = tokio::time::Instant::now();
            if now.duration_since(last_fps_checked).as_secs() >= 1 {
                let _ = fps_sender.send(frame_count);
                frame_count = 0;
                last_fps_checked = now;
            }
        }
    }

    pub async fn handle_disconnect(
        session_key: u32,
        sessions_guard: &mut tokio::sync::RwLockWriteGuard<
            '_,
            HashMap<u32, Arc<tokio::sync::RwLock<Session>>>,
        >,
    ) {
        debug!("Session #{} is disconnected", session_key);

        let session = sessions_guard.get_mut(&session_key);
        if session.is_none() {
            warn!("Session not found: {}", session_key);
            return;
        }

        let mut session = session.unwrap().write().await;
        session.message_inbound.clear();
        session.message_outbound.clear();
        drop(session);

        sessions_guard.remove(&session_key);
    }

    async fn process_update(&self, delta_time: f32) {
        let mut sessions_writelock = self.sessions.write().await;
        let mut objects_writelock = self.game_objects.write().await;

        // 1단계: 메시지 수집
        let mut messages_to_process: HashMap<u32, Vec<MsgIn>> = HashMap::new();

        // 클라이언트를 순회하며 살아있는 클라이언트의 메시지 처리
        for (session_key, session) in sessions_writelock.iter() {
            let mut session = session.write().await;
            while let Some(message) = session.message_inbound.pop_front() {
                messages_to_process
                    .entry(*session_key)
                    .or_insert(Vec::new())
                    .push(message);
            }
        }

        // 1-1단계: 메시지 처리
        for (session_key, messages) in messages_to_process.iter() {
            for message in messages.iter() {
                self.process_message(
                    message.clone(),
                    *session_key,
                    &mut sessions_writelock,
                    &mut objects_writelock,
                )
                .await;
            }
        }

        // 2단계: 게임 로직 처리
        // state_writelock 의 game_objects 를 순회하면서 update 함수를 호출한다
        for (_key, game_object) in objects_writelock.iter() {
            let mut write_lock = game_object.write().await;

            let _result = write_lock
                .update(&mut sessions_writelock, &objects_writelock, delta_time)
                .await;
        }

        // 3단계: 끊긴 세션 처리
        let mut disconnected_sessions: Vec<u32> = Vec::new();
        for (session_key, session) in sessions_writelock.iter() {
            let session = session.read().await;
            if !session.alive {
                disconnected_sessions.push(*session_key);
            }
        }
        for session_key in disconnected_sessions.iter() {
            Self::handle_disconnect(*session_key, &mut sessions_writelock).await;
        }

        drop(objects_writelock);
        drop(sessions_writelock);
    }

    pub async fn process_message(
        &self,
        message: MsgIn,
        session_key: u32,
        mut sessions_writelock: &mut tokio::sync::RwLockWriteGuard<'_, Sessions>,
        mut objects_writelock: &mut tokio::sync::RwLockWriteGuard<'_, GameObjects>,
    ) {
        match message {
            MsgIn::Echo(data) => {
                let handle_result = handle_echo(
                    &data,
                    session_key,
                    &mut sessions_writelock,
                    &mut objects_writelock,
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

    async fn shutdown_signal(terminate_sender: watch::Sender<bool>) {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                info!("Ctrl+C received, starting graceful shutdown");
            },
            _ = terminate => {
                info!("SIGTERM received, starting graceful shutdown");
            },
        }

        terminate_sender
            .send(true)
            .expect("failed to send terminate signal");
    }
}
