use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HookAction {
    #[serde(rename = "subscribe")]
    Subscribe,
    #[serde(rename = "unsubscribe")]
    Unsubscribe,
    #[serde(other)]
    Other,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HookBody {
    #[serde(rename = "type")]
    pub action: HookAction,
    #[serde(rename = "data[email]")]
    pub email: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetIdForEmailResponse {
    pub id: String,
}
