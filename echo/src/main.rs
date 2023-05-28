use common::message::{Message, MessageBody, MessageId};
use common::node::{Node, NodeId};
use common::runtime::Runtime;
use tokio::sync::mpsc::UnboundedSender;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum MessagePayload {
    Echo { echo: String },
    EchoOk { echo: String },
}

struct EchoNode {
    id: NodeId,
    curr_msg_id: MessageId,
    tx: UnboundedSender<Message<MessagePayload>>,
}

impl<'a> Node<'a> for EchoNode {
    type Payload = MessagePayload;

    fn handle_message(&mut self, message: common::message::Message<Self::Payload>) {
        match message.body.payload {
            Self::Payload::Echo { echo } => {
                let next_msg_id = self.next_msg_id();

                self.tx
                    .send(Message {
                        src: self.id.clone(),
                        dest: message.src,
                        body: MessageBody {
                            msg_id: Some(next_msg_id),
                            in_reply_to: message.body.msg_id,
                            payload: Self::Payload::EchoOk { echo },
                        },
                    })
                    .expect("failed sending message");
            }
            Self::Payload::EchoOk { .. } => {}
        }
    }

    fn from_init(
        node_id: NodeId,
        _neighbors: Vec<NodeId>,
        tx: tokio::sync::mpsc::UnboundedSender<common::message::Message<Self::Payload>>,
    ) -> Self {
        Self {
            id: node_id,
            curr_msg_id: 0,
            tx,
        }
    }

    fn next_msg_id(&mut self) -> MessageId {
        self.curr_msg_id = self.curr_msg_id.checked_add(1).expect("ids exhausted");
        self.curr_msg_id
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    Runtime::start::<EchoNode, _, _>(stdin, stdout).await;
}
