pub use serde::{Deserialize, Serialize};

/**
 * MsgInAuthData
 * - user_identifier: String
 *  - The unique token of the user
 */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MsgInAuthData {
    pub user_identifier: String,
    pub device_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MsgOutAuthData {
    pub success: bool,
    pub access_token: String,
}
