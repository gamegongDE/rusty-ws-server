use super::msg_echo::MsgInEchoData;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum MsgIn {
    Echo(MsgInEchoData),
}
