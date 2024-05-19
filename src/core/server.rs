use crate::core::session::Session;
use crate::handler::handler_echo::handle_echo;
use crate::system::map::GameMap;
use crate::msg::msg_in::MsgIn;
use crate::object::base::GameObjectTrait;
use futures::{join, StreamExt};
use log::{debug, error, info, warn};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
use tokio::signal;
use std::sync::atomic::AtomicU32;
#[allow(unused_imports)]
#[cfg(target_os = "windows")]
use tokio::signal::windows::ctrl_c;
use tokio::sync::{mpsc, watch, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};
use warp::Filter;

pub type Sessions = HashMap<u32, Session>;
pub type SessionsArc = Arc<RwLock<Sessions>>;
pub type GameObjects = HashMap<u32, Arc<RwLock<Box<dyn GameObjectTrait>>>>;
pub type GameObjectsArc = Arc<RwLock<GameObjects>>;
pub type GameMapArc = Arc<RwLock<GameMap>>;

#[allow(dead_code)]
#[derive(Clone)]
pub(crate) struct GameServer {
    listen_address: SocketAddr,
    timestamp_server_start: u64,
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
            sessions,
            game_objects,
            game_map,
            next_session_key,
            next_object_key,
        })
    }

    pub async fn run(&mut self) -> Result<(), String> {
        info!(
            "Game WebSocket server listening on: {}",
            self.listen_address
        );

        let sessions_clone = self.sessions.clone();
        let session_warp = warp::any().map(move || sessions_clone.clone());
        let session_key_clone = self.next_session_key.clone();
        let session_key_warp = warp::any().map(move || session_key_clone.clone());

        let command_handler = warp::path("command").map(|| "de game command handler");

        let intro = warp::path::end().map(|| "de game server");

        let (terminate_sender, terminate_receiver) = watch::channel(false);
        // shutdown_signal 태스크를 생성합니다.
        let _ = tokio::spawn(async {
            info!("listening to shutdown signal...");
            Self::shutdown_signal(terminate_sender).await;
        });

        // GET /ws
        let ws = warp::path("ws")
            .and(warp::ws())
            .and(warp::addr::remote())
            .and(session_warp)
            .and(session_key_warp)
            .map(|ws: warp::ws::Ws, addr: Option<SocketAddr>, sessions, next_session_key| {
                ws.on_upgrade(move |socket| Self::handle_connection(socket, addr, sessions, next_session_key))
            });

        let routes = ws.or(command_handler).or(intro);

        let mut copied_terminate_receiver = terminate_receiver.clone();
        let (_, server) =
            warp::serve(routes).bind_with_graceful_shutdown(self.listen_address, async move {
                copied_terminate_receiver.changed().await.unwrap();
                info!("Shutting down server...");
            });

        self.timestamp_server_start = chrono::Utc::now().timestamp_millis() as u64;
        info!("Running server at {}!", self.listen_address);
        info!("current timestamp: {}", self.timestamp_server_start);

        // 웹소켓 서버를 시작합니다.
        let tokio_server_handler = tokio::spawn(server);

        // 로직 루프를 실행합니다.
        let mut last_update = tokio::time::Instant::now();
        let mut last_fps = 0;
        let mut last_fps_checked = tokio::time::Instant::now();
        let target_fps = 60.0;
        let mut next_target_frame_time = tokio::time::Instant::now() + Duration::from_secs_f32(1.0 / target_fps);
        
        let copied_terminate_receiver = terminate_receiver.clone();
        loop {
            let now = tokio::time::Instant::now();
            let delta_time = now.duration_since(last_update).as_secs_f32();
            last_update = now;

            self.update(delta_time).await;

            last_fps += 1;
            
            if last_fps_checked.elapsed().as_millis() >= 1000 {
                last_fps_checked = now;
                debug!("FPS: {}", last_fps);
                last_fps = 0;
            }

            if *copied_terminate_receiver.borrow() {
                info!("Terminate signal received");
                break;
            }

            let sleep_time = next_target_frame_time.duration_since(now);
            if sleep_time.as_secs_f32() > 0.0 {
                tokio::time::sleep(sleep_time).await;
            }
            next_target_frame_time = next_target_frame_time + Duration::from_secs_f32(1.0 / target_fps);
        }

        let _ = join!(tokio_server_handler);

        info!("Finished server at {}!", self.listen_address);

        Ok(())
    }

    /**
     * handle new websocket connection
     */
    pub async fn handle_connection(
        ws: WebSocket,
        addr: Option<SocketAddr>,
        sessions: SessionsArc,
        session_key: Arc<AtomicU32>,
    ) {
        let session_key = session_key.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        info!("Welcome #{}, from {}", session_key, addr.unwrap());

        let (user_tx, mut user_rx) = ws.split();
        let (tx, rx) = mpsc::unbounded_channel();

        let rx = UnboundedReceiverStream::new(rx);

        tokio::spawn(rx.forward(user_tx));

        let session = Session {
            key: session_key,
            address: addr,
            session: "".to_string(),
            channel: tx,
            message_inbound: VecDeque::new(),
            message_outbound: VecDeque::new(),
            id: 0, // unauthorized now
        };

        debug!("RWLOCK clients at handle_connection");
        sessions.write().await.insert(session_key, session);
        debug!("RWLOCK clients at handle_connection OK");

        while let Some(result) = user_rx.next().await {
            let _ = match result {
                Ok(msg) => {
                    Self::handle_message(session_key, msg, sessions.clone()).await;
                }
                Err(e) => {
                    warn!("Error receiving message: {}", e);
                    break;
                }
            };
        }

        Self::handle_disconnect(session_key, sessions.clone()).await;
    }

    pub async fn handle_message(session_key: u32, message: Message, sessions: SessionsArc) {
        if !message.is_text() {
            return;
        }
        let message_parse: Result<MsgIn, _> = serde_json::from_str(message.to_str().unwrap());
        if message_parse.is_err() {
            warn!("Message is not valid json : {}", message.to_str().unwrap());
            return;
        }
        let message = message_parse.unwrap();
        debug!("Message: {:?}", message);

        // find sessions by key
        let mut sessions_writelock = sessions.write().await;
        {
            let sessions = sessions_writelock.get_mut(&session_key);
            if sessions.is_none() {
                warn!("Session not found: {}", session_key);
                return;
            }

            // add message in session inbound message list
            sessions.unwrap().message_inbound.push_back(message.clone());
        }
        drop(sessions_writelock);
    }

    pub async fn handle_disconnect(session_key: u32, sessions: SessionsArc) {
        info!("Bye #{}", session_key);

        let mut sessions_writelock = sessions.write().await;
        let session = sessions_writelock.get_mut(&session_key);
        if session.is_none() {
            warn!("Session not found: {}", session_key);
            return;
        }

        let session = session.unwrap();
        session.message_inbound.clear();
        session.message_outbound.clear();
        sessions_writelock.remove(&session_key);

        info!("Session #{} is disconnected", session_key);
    }

    async fn update(&self, delta_time: f32) {
        // info!("update");

        let mut sessions_writelock = self.sessions.write().await;
        let mut objects_writelock = self.game_objects.write().await;

        // 1단계: 메시지 수집
        let mut messages_to_process: HashMap<u32, Vec<MsgIn>> = HashMap::new();

        // 클라이언트를 순회하며 살아있는 클라이언트의 메시지 처리
        for (session_key, session) in sessions_writelock.iter_mut() {
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
                .update(&mut sessions_writelock, &objects_writelock, delta_time).await;
        }

        // 3단계: 메시지 전송
        for (_, session) in sessions_writelock.iter_mut() {
            for message in session.message_outbound.drain(..) {
                let message_json =
                    serde_json::to_string(&message).expect("Failed to serialize message");
                session
                    .channel
                    .send(Ok(Message::text(message_json)))
                    .expect("Failed to send message");
            }
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

        info!("listening to shutdown");
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
