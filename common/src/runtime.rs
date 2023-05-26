use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};

use crate::message::{Message, MessageBody, MessagePayload};

use crate::node::Node;

pub struct Runtime;

impl Runtime {
    /// Starts delegating processing of messages as they arrive on the specified `reader`
    pub async fn start<N, R, W>(reader: R, mut writer: W)
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Send + Unpin + 'static,
        N: Node,
    {
        let reader = BufReader::new(reader);
        let mut lines = reader.lines();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                let mut bytes = serde_json::to_vec(&message).expect("failed serializing message");
                bytes.extend(b"\n");
                writer.write_all(&bytes).await.expect("failed writing buf");
            }
        });

        // Handle init message
        let startup = lines
            .next_line()
            .await
            .expect("error reading from input stream!");
        let mut n: N;

        if let Some(init) = startup {
            let message: Message =
                serde_json::from_str(init.as_str()).expect("failed to deserialize init message!");
            match message.body.payload {
                MessagePayload::Init { node_id, node_ids } => {
                    n = N::from_init(node_id.clone(), node_ids, tx.clone());
                    let next_id = n.next_msg_id();
                    tx.send(Message {
                        src: node_id,
                        dest: message.src,
                        body: MessageBody {
                            msg_id: Some(next_id),
                            in_reply_to: message.body.msg_id,
                            payload: MessagePayload::InitOk,
                        },
                    })
                    .expect("failed to send init ok message");
                }
                _ => panic!("first message was not init message"),
            }
        } else {
            panic!("expected init message")
        }

        while let Some(line) = lines
            .next_line()
            .await
            .expect("error reading from input stream!")
        {
            let message: Message = serde_json::from_str(line.as_str())
                .expect("failed to deserialize line from input stream!");
            n.handle_message(message);
        }
    }
}
