use crate::session::Session;
use crate::map::GameMap;
use crate::msg::msg_in::MsgIn;
use crate::object::base::GameObjectTrait;
use crate::object::player::PlayerGameObject;
use crate::update::UpdateThread;
use futures::{join, StreamExt};
use log::{debug, info, warn};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::{collections::HashMap, sync::Arc};
use tokio::signal;
#[cfg(target_os = "windows")]
use tokio::signal::windows::ctrl_c;
use tokio::sync::{mpsc, watch, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};
use warp::Filter;

static NEXT_OBJECT_KEY: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1000);
pub type ClientMap = HashMap<u32, Session>;
pub type SharedClients = Arc<RwLock<ClientMap>>;
pub type SharedState = Arc<RwLock<GameState>>;
pub type PlayerObjects = HashMap<u32, Arc<RwLock<PlayerGameObject>>>;
pub type GameObjects = HashMap<u32, Arc<RwLock<Box<dyn GameObjectTrait>>>>;

#[allow(dead_code)]
pub struct GameState {
    pub timestamp_game_start: u64,
    pub game_map: GameMap,
    pub player_objects: PlayerObjects,
    pub game_objects: GameObjects,
}

impl GameState {
    fn new(current_timestamp: u64) -> Self {
        Self {
            timestamp_game_start: current_timestamp,
            game_map: GameMap::new(),
            player_objects: HashMap::new(),
            game_objects: HashMap::new(),
        }
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct GameServer {
    listen_address: SocketAddr,
    clients: SharedClients,
    state: SharedState,
    timestamp_server_start: u64,
}

impl GameServer {
    pub async fn new(addr: SocketAddr) -> Result<Self, std::io::Error> {
        let listen_address = addr;
        let clients = SharedClients::default();
        let state = GameState::new(tokio::time::Instant::now().elapsed().as_millis() as u64);
        Ok(Self {
            listen_address,
            clients,
            timestamp_server_start: 0,
            state: Arc::new(RwLock::new(state)),
        })
    }

    pub async fn run(&mut self) -> Result<(), String> {
        info!(
            "Game WebSocket server listening on: {}",
            self.listen_address
        );

        let clients = self.clients.clone();
        let clients = warp::any().map(move || clients.clone());

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
            .and(clients)
            .map(|ws: warp::ws::Ws, addr: Option<SocketAddr>, clients| {
                ws.on_upgrade(move |socket| Self::handle_connection(socket, addr, clients))
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

        // 로직 스레드를 실행합니다.

        let copied_state = self.state.clone();
        let copied_clients = self.clients.clone();
        let copied_terminate_receiver = terminate_receiver.clone();
        let update_thread_handler = tokio::spawn(async move {
            let mut update_thread = UpdateThread::new(copied_clients, copied_state);
            let _ = update_thread.run(copied_terminate_receiver).await;
            info!("update thread ended");
        });

        let (_, _) = join!(tokio_server_handler, update_thread_handler);

        info!("Finished server at {}!", self.listen_address);

        Ok(())
    }

    pub async fn handle_connection(
        ws: WebSocket,
        addr: Option<SocketAddr>,
        clients: SharedClients,
    ) {
        let new_user_key = NEXT_OBJECT_KEY.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        info!("Welcome #{}, from {}", new_user_key, addr.unwrap());

        let (user_tx, mut user_rx) = ws.split();
        let (tx, rx) = mpsc::unbounded_channel();

        let rx = UnboundedReceiverStream::new(rx);

        tokio::spawn(rx.forward(user_tx));

        let client = Session {
            key: new_user_key,
            address: addr,
            session: "".to_string(),
            channel: tx,
            message_inbound: VecDeque::new(),
            message_outbound: VecDeque::new(),
            id: new_user_key,
            alive: true,
        };

        debug!("RWLOCK clients at handle_connection");
        clients.write().await.insert(new_user_key, client);
        debug!("RWLOCK clients at handle_connection OK");

        while let Some(result) = user_rx.next().await {
            let _ = match result {
                Ok(msg) => {
                    Self::handle_message(new_user_key, msg, clients.clone()).await;
                }
                Err(e) => {
                    warn!("Error receiving message: {}", e);
                    break;
                }
            };
        }

        Self::disconnect(new_user_key, clients.clone()).await;
    }

    pub async fn handle_message(client_key: u32, message: Message, clients: SharedClients) {
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

        // find client by key
        debug!("RWLOCK clients at handle_message");
        let mut clients_writelock = clients.write().await;
        debug!("RWLOCK clients at handle_message OK");
        {
            let client = clients_writelock.get_mut(&client_key);
            if client.is_none() {
                warn!("Client not found: {}", client_key);
                return;
            }

            // add message in client inbound message list
            client.unwrap().message_inbound.push_back(message.clone());
        }
        drop(clients_writelock);
    }

    pub async fn disconnect(client_key: u32, clients: SharedClients) {
        info!("Bye #{}", client_key);

        debug!("RWLOCK clients at disconnect");
        let mut clients_write_lock = clients.write().await;
        debug!("RWLOCK clients at disconnect OK");
        let client = clients_write_lock.get_mut(&client_key);
        if client.is_none() {
            warn!("Client not found: {}", client_key);
            return;
        }

        let client = client.unwrap();
        client.message_inbound.clear();
        client.message_outbound.clear();
        client.alive = false;
        info!("Client #{} is disconnected", client_key);
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
