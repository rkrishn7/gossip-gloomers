use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub type MessageId = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message<P> {
    pub src: String,
    pub dest: String,
    pub body: MessageBody<P>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBody<P> {
    pub msg_id: Option<MessageId>,
    pub in_reply_to: Option<MessageId>,
    #[serde(flatten)]
    pub payload: P,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum MessagePayload {
    Init {
        node_id: String,
        node_ids: Vec<String>,
    },
    InitOk,
    Echo {
        echo: String,
    },
    Generate,
    GenerateOk {
        id: String,
    },
    EchoOk {
        echo: String,
    },
    Broadcast {
        message: i32,
    },
    BroadcastOk,
    Read,
    ReadOk {
        messages: Vec<i32>,
        // value: u32,
    },
    Topology {
        topology: HashMap<String, Vec<String>>,
    },
    TopologyOk,
    Add {
        delta: u32,
    },
    AddOk,
}
