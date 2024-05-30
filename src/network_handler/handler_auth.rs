use fred::clients::RedisClient;
use tokio::sync::RwLockWriteGuard;

use crate::core::server::{GameObjects, Sessions};
use crate::network_message::msg_auth::{MsgInAuthData, MsgOutAuthData};
use crate::network_message::msg_out::MsgOut;

#[allow(unused_variables)]
pub async fn handle_auth(
    msg: &MsgInAuthData,
    sender_key: u32,
    sessions_writelock: &mut RwLockWriteGuard<'_, Sessions>,
    objects_writelock: &mut RwLockWriteGuard<'_, GameObjects>,
    redis_client: Option<RedisClient>,
) -> Result<(), String> {
    if msg.user_identifier.is_empty() {
        return Err("user_identifier empty".to_string());
    }

    // use brackets to limit the write lock scope
    {
        let sender_result = sessions_writelock.get(&sender_key);
        if sender_result.is_none() {
            return Err("Sender not found".to_string());
        }
        let mut sender = sender_result.unwrap().write().await;

        if msg.user_identifier.is_empty() || msg.user_identifier.len() < 6 {
            let message = MsgOutAuthData {
                success: false,
                access_token: "".to_string(),
            };

            sender.message_outbound.push_back(MsgOut::OnAuth(message));
            return Ok(());
        }

        let message = MsgOutAuthData {
            success: true,
            access_token: "access_token".to_string(),
        };

        sender.message_outbound.push_back(MsgOut::OnAuth(message));
    }

    Ok(())
}
