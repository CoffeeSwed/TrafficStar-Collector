use serde::{Deserialize, Serialize};

#[derive(Debug,PartialEq, Eq,strum_macros::EnumMessage, Serialize, Deserialize)]
pub enum HttpProxyCommands {
    Startedslave,
    Stop,
    Stopped,
    Restart,
}