use tokio::sync::mpsc::UnboundedSender;

use crate::message::{Message, MessageId};

pub type NodeId = String;

pub trait Node {
    fn handle_message(&mut self, message: Message);
    fn from_init(node_id: NodeId, neighbors: Vec<NodeId>, tx: UnboundedSender<Message>) -> Self;
    fn next_msg_id(&mut self) -> MessageId;
}
