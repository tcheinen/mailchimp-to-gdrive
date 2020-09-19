use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};

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

pub struct StringRejection {
    pub message: String,
}

impl StringRejection {
    pub fn new(message: &str) -> Self {
        StringRejection {
            message: message.to_string(),
        }
    }
}

impl Debug for StringRejection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)?;
        Ok(())
    }
}

impl warp::reject::Reject for StringRejection {}

#[derive(Serialize)]
pub struct ErrorMessage {
    pub code: u16,
    pub message: String,
}
