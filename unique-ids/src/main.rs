use common::message::{Message, MessageBody, MessageId, MessagePayload};
use common::node::{Node, NodeId};
use common::runtime::Runtime;
use tokio::sync::mpsc::UnboundedSender;

struct UniqueIdNode {
    id: NodeId,
    curr_msg_id: MessageId,
    tx: UnboundedSender<Message>,
}

impl Node for UniqueIdNode {
    fn handle_message(&mut self, message: common::message::Message) {
        match message.body.payload {
            MessagePayload::Generate => {
                let msg_id = self.next_msg_id();

                // Since message IDs are guaranteed unique per node, we can prefix them with
                // the node ID to create a globally unique ID in the cluster
                let id = format!("{}-{}", self.id, msg_id);

                self.tx
                    .send(Message {
                        src: self.id.clone(),
                        dest: message.src,
                        body: MessageBody {
                            msg_id: Some(msg_id),
                            in_reply_to: message.body.msg_id,
                            payload: MessagePayload::GenerateOk { id },
                        },
                    })
                    .expect("failed sending generate reply");
            }
            MessagePayload::GenerateOk { .. } => {}
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
            curr_msg_id: Default::default(),
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

    Runtime::start::<UniqueIdNode, _, _>(stdin, stdout).await;
}
