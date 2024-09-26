use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cmd")]
pub enum IncomingMessage {
    #[serde(rename = "login")]
    Login(LoginCommand),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginCommand {
    pub user_token: String,
    pub device_token: String,
    pub secret_key: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum OutgoingMessage {
    #[serde(rename = "device_online")]
    DeviceOnline,

    #[serde(rename = "device_offline")]
    DeviceOffline,
}
