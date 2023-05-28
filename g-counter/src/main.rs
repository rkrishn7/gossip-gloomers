use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use common::message::{Message, MessageBody, MessageId};
use common::node::{Node, NodeId};
use common::runtime::Runtime;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum MessagePayload {
    Read,
    ReadOk { value: u32 },
    Add { delta: u32 },
    AddOk,
}

#[derive(Debug, Clone)]
struct GCounterNode {
    id: NodeId,
    counter: u32,
    neighbors: Vec<NodeId>,
    curr_msg_id: MessageId,
    tx: UnboundedSender<Message<MessagePayload>>,
    unacked_messages: Arc<Mutex<BTreeSet<MessageId>>>,
}

impl<'de> Node<'de> for GCounterNode {
    type Payload = MessagePayload;

    fn handle_message(&mut self, message: Message<Self::Payload>) {
        if self.id != message.dest {
            return;
        }

        match message.body.payload {
            MessagePayload::Add { delta } => {
                self.counter += delta;
                let msg_id = self.next_msg_id();

                for neighbor in self.neighbors.clone() {
                    let msg_id = self.next_msg_id();
                    let node_id = self.id.clone();
                    let tx = self.tx.clone();
                    let unacked_msgs = Arc::clone(&self.unacked_messages);

                    {
                        self.unacked_messages
                            .lock()
                            .expect("poisoned lock")
                            .extend(std::iter::once(msg_id));
                    }

                    tokio::spawn(async move {
                        loop {
                            if !unacked_msgs
                                .lock()
                                .expect("poisoned lock")
                                .contains(&msg_id)
                            {
                                break;
                            }
                            tx.send(Message {
                                src: node_id.clone(),
                                dest: neighbor.clone(),
                                body: MessageBody {
                                    msg_id: Some(msg_id),
                                    in_reply_to: None,
                                    payload: MessagePayload::Add { delta },
                                },
                            })
                            .expect("failed sending message");

                            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                        }
                    });
                }

                self.tx
                    .send(Message {
                        src: self.id.clone(),
                        dest: message.src,
                        body: MessageBody {
                            msg_id: Some(msg_id),
                            in_reply_to: message.body.msg_id,
                            payload: MessagePayload::AddOk,
                        },
                    })
                    .expect("failed sending message");
            }
            MessagePayload::Read => {
                let msg_id = self.next_msg_id();

                self.tx
                    .send(Message {
                        src: self.id.clone(),
                        dest: message.src,
                        body: MessageBody {
                            msg_id: Some(msg_id),
                            in_reply_to: message.body.msg_id,
                            payload: MessagePayload::ReadOk {
                                value: self.counter,
                            },
                        },
                    })
                    .expect("failed sending message");
            }
            MessagePayload::AddOk => {
                if let Some(in_reply_to) = &message.body.in_reply_to {
                    self.unacked_messages
                        .lock()
                        .expect("poisoned lock")
                        .remove(in_reply_to);
                }
            }
            MessagePayload::ReadOk { .. } => {}
        }
    }

    fn from_init(
        node_id: NodeId,
        neighbors: Vec<NodeId>,
        tx: tokio::sync::mpsc::UnboundedSender<Message<Self::Payload>>,
    ) -> Self {
        Self {
            id: node_id,
            counter: Default::default(),
            neighbors,
            curr_msg_id: Default::default(),
            unacked_messages: Arc::new(Mutex::new(BTreeSet::new())),
            tx,
        }
    }

    fn next_msg_id(&mut self) -> MessageId {
        self.curr_msg_id = self.curr_msg_id.checked_add(1).expect("id overflow");
        self.curr_msg_id
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    Runtime::start::<GCounterNode, _, _>(stdin, stdout).await;
}
