use super::{msg_auth::MsgInAuthData, msg_ping::MsgInPingData};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum MsgIn {
    Ping(MsgInPingData),
    Auth(MsgInAuthData),
}
