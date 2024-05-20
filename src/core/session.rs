use std::{collections::VecDeque, net::SocketAddr};
use tokio::sync::mpsc;
use warp::ws::Message;

use crate::msg::{msg_in::MsgIn, msg_out::MsgOut};

#[allow(dead_code)]
pub struct Session {
    pub(crate) key: u32,
    pub(crate) address: Option<SocketAddr>,
    pub(crate) session: String,
    pub(crate) channel: mpsc::UnboundedSender<Result<Message, warp::Error>>,
    pub(crate) message_inbound: VecDeque<MsgIn>,
    pub(crate) message_outbound: VecDeque<MsgOut>,
    pub(crate) user_index: Option<u32>,
    pub(crate) user_access_token: Option<String>,
    pub(crate) player_object_key: Option<u32>,
}

impl Session {
}
