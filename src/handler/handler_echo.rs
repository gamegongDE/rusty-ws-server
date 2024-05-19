use tokio::sync::RwLockWriteGuard;
use warp::filters::ws::Message;

use crate::core::server::{GameObjects, Sessions};
use crate::msg::msg_echo::{MsgInEchoData, MsgOutEchoData};

use crate::msg::msg_out::MsgOut;

#[allow(unused_variables)]
pub async fn handle_echo(
    msg: &MsgInEchoData,
    sender_key: u32,
    sessions_writelock: &RwLockWriteGuard<'_, Sessions>,
    objects_writelock: &RwLockWriteGuard<'_, GameObjects>,
) -> Result<(), String> {
    let message = MsgOutEchoData {
        message: format!("-> {}", msg.message),
    };
    let json_message =
        serde_json::to_string(&MsgOut::OnEcho(message)).expect("Failed to serialize message");

    let sender_result = sessions_writelock.get(&sender_key);
    if sender_result.is_none() {
        return Err("Sender not found".to_string());
    }
    let sender = sender_result.unwrap();

    sender
        .channel
        .send(Ok(Message::text(&json_message)))
        .expect("Failed to send message");

    Ok(())
}
