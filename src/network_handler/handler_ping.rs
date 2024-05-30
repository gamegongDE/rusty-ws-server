use tokio::sync::RwLockWriteGuard;

use crate::core::server::{GameObjects, Sessions};
use crate::network_message::msg_out::MsgOut;
use crate::network_message::msg_ping::{MsgInPingData, MsgOutPongData};

#[allow(unused_variables)]
pub async fn handle_ping(
    msg: &MsgInPingData,
    sender_key: u32,
    sessions_writelock: &RwLockWriteGuard<'_, Sessions>,
    objects_writelock: &RwLockWriteGuard<'_, GameObjects>,
) -> Result<(), String> {
    let sender_result = sessions_writelock.get(&sender_key);
    if sender_result.is_none() {
        return Err("Sender not found".to_string());
    }
    let mut sender = sender_result.unwrap().write().await;

    let message = MsgOutPongData {
        timestamp: msg.timestamp,
    };

    sender
        .message_outbound
        .push_back(MsgOut::OnEcho(message.clone()));

    Ok(())
}
