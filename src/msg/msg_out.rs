use super::msg_echo::MsgOutEchoData;
use serde::{Deserialize, Serialize};

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum MsgOut {
    OnEcho(MsgOutEchoData),
}