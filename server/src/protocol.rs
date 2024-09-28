use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cmd")]
pub enum IncomingMessage {
    #[serde(rename = "login")]
    Login(LoginCommand),

    #[serde(rename = "sync")]
    Sync { device_pts: u64 },

    #[serde(rename = "send_message")]
    SendMessage(SendMessageCommand),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginCommand {
    pub user_token: String,
    pub device_token: String,
    pub secret_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageCommand {
    pub receiver: String,
    pub contents: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum OutgoingMessage {
    #[serde(rename = "device_online")]
    DeviceOnline,

    #[serde(rename = "device_offline")]
    DeviceOffline,

    #[serde(rename = "sync_updates")]
    SyncUpdates(SyncUpdates),

    #[serde(rename = "update")]
    Update(Update),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncUpdates {
    pub too_long: bool,
    pub synced: bool,
    pub updates: Vec<Update>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Update {
    pub pts: u64,
    pub payload: UpdatePayload,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UpdatePayload {
    #[serde(rename = "new_message")]
    NewMessage(NewMessageUpdate),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewMessageUpdate {
    pub sender: String,
    pub contents: String,
}
