use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex};

use common::message::{Message, MessageBody, MessageId, MessagePayload};
use common::node::{Node, NodeId};
use common::runtime::Runtime;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone)]
struct BroadcastNode {
    id: NodeId,
    seen: Vec<i32>,
    topology: Option<HashMap<NodeId, Vec<NodeId>>>,
    curr_msg_id: MessageId,
    tx: UnboundedSender<Message>,
    unacked_messages: Arc<Mutex<BTreeSet<MessageId>>>,
}

impl Node for BroadcastNode {
    fn handle_message(&mut self, message: common::message::Message) {
        if self.id != message.dest {
            return;
        }

        match message.body.payload {
            MessagePayload::Broadcast { message: msg } => {
                if self.seen.contains(&msg) {
                    return;
                }

                self.seen.push(msg);

                let node_neighbors = self
                    .topology
                    .as_ref()
                    .expect("topology unset")
                    .get(&self.id)
                    .expect("unknown node")
                    .clone();

                for neighbor in node_neighbors {
                    if message.src == neighbor {
                        continue;
                    }

                    let msg_id = self.next_msg_id();
                    let tx = self.tx.clone();
                    let node_id = self.id.clone();
                    let unacked_msgs = Arc::clone(&self.unacked_messages);
                    {
                        unacked_msgs
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
                                    payload: MessagePayload::Broadcast { message: msg },
                                },
                            })
                            .expect("failed sending message");

                            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                        }
                    });
                }

                let msg_id = self.next_msg_id();

                self.tx
                    .send(Message {
                        src: self.id.clone(),
                        dest: message.src,
                        body: MessageBody {
                            msg_id: Some(msg_id),
                            in_reply_to: message.body.msg_id,
                            payload: MessagePayload::BroadcastOk,
                        },
                    })
                    .expect("failed sending message");
            }
            MessagePayload::Topology { topology } => {
                self.topology = Some(topology);

                let msg_id = self.next_msg_id();

                self.tx
                    .send(Message {
                        src: self.id.clone(),
                        dest: message.src,
                        body: MessageBody {
                            msg_id: Some(msg_id),
                            in_reply_to: message.body.msg_id,
                            payload: MessagePayload::TopologyOk,
                        },
                    })
                    .expect("failed sending message");
            }
            MessagePayload::Read => {
                let msg_id = { self.next_msg_id() };

                let seen = &self.seen;

                self.tx
                    .send(Message {
                        src: self.id.clone(),
                        dest: message.src,
                        body: MessageBody {
                            msg_id: Some(msg_id),
                            in_reply_to: message.body.msg_id,
                            payload: MessagePayload::ReadOk {
                                messages: seen.clone(),
                            },
                        },
                    })
                    .expect("failed sending message");
            }
            MessagePayload::BroadcastOk => {
                self.unacked_messages
                    .lock()
                    .expect("poisoned lock")
                    .remove(&message.body.in_reply_to.unwrap());
            }
            MessagePayload::ReadOk { .. } | MessagePayload::TopologyOk => {}
            _ => unimplemented!(),
        }
    }

    fn from_init(
        node_id: NodeId,
        _neighbors: Vec<NodeId>,
        tx: tokio::sync::mpsc::UnboundedSender<common::message::Message>,
    ) -> Self {
        Self {
            id: node_id,
            seen: vec![],
            topology: None,
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
    console_subscriber::init();
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    Runtime::start::<BroadcastNode, _, _>(stdin, stdout).await;
}
