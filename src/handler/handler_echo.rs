use tokio::sync::RwLockWriteGuard;

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
    // 0. 유저의 데이터를 가져온다
    let sender_result = sessions_writelock.get(&sender_key);
    if sender_result.is_none() {
        return Err("Sender not found".to_string());
    }
    let mut sender = sender_result.unwrap().write().await;

    let message = MsgOutEchoData {
        message: msg.message.clone(),
    };

    sender
        .message_outbound
        .push_back(MsgOut::OnEcho(message.clone()));

    Ok(())
}
