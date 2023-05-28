use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

use crate::message::{Message, MessageId};

pub type NodeId = String;

pub trait Node<'de> {
    type Payload: Serialize + Deserialize<'de> + Send;

    fn handle_message(&mut self, message: Message<Self::Payload>);
    fn from_init(
        node_id: NodeId,
        neighbors: Vec<NodeId>,
        tx: UnboundedSender<Message<Self::Payload>>,
    ) -> Self;
    fn next_msg_id(&mut self) -> MessageId;
}
