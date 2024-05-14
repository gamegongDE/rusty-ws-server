pub use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MsgInEchoData {
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MsgOutEchoData {
    pub message: String,
}
