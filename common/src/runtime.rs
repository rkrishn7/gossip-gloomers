use crate::message::{Message, MessageBody};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::node::Node;

pub struct Runtime;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum BootstrapPayload {
    Init {
        node_id: String,
        node_ids: Vec<String>,
    },
    InitOk,
}

impl Runtime {
    /// Starts delegating processing of messages as they arrive on the specified `reader`
    pub async fn start<N, R, W>(reader: R, mut writer: W)
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Send + Unpin + 'static,
        N: for<'a> Node<'a> + 'static,
    {
        let reader = BufReader::new(reader);
        let mut lines = reader.lines();
        let (bootstrap_tx, bootstrap_rx) =
            tokio::sync::mpsc::unbounded_channel::<Message<BootstrapPayload>>();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Message<N::Payload>>();

        tokio::spawn(async move {
            async fn write_loop<W, T>(writer: &mut W, mut rx: UnboundedReceiver<T>)
            where
                T: Serialize,
                W: AsyncWrite + Send + Unpin + 'static,
            {
                while let Some(message) = rx.recv().await {
                    let mut bytes =
                        serde_json::to_vec(&message).expect("failed serializing message");
                    bytes.extend(b"\n");
                    writer.write_all(&bytes).await.expect("failed writing buf");
                }
            }

            write_loop(&mut writer, bootstrap_rx).await;
            write_loop(&mut writer, rx).await;
        });

        // Handle init message
        let startup = lines
            .next_line()
            .await
            .expect("error reading from input stream!");
        let mut n: N;

        if let Some(init) = startup {
            let message: Message<BootstrapPayload> =
                serde_json::from_str(init.as_str()).expect("failed to deserialize init message!");
            match message.body.payload {
                BootstrapPayload::Init { node_id, node_ids } => {
                    n = N::from_init(
                        node_id.clone(),
                        node_ids.into_iter().filter(|id| *id != node_id).collect(),
                        tx.clone(),
                    );
                    let next_id = n.next_msg_id();
                    bootstrap_tx
                        .send(Message {
                            src: node_id,
                            dest: message.src,
                            body: MessageBody {
                                msg_id: Some(next_id),
                                in_reply_to: message.body.msg_id,
                                payload: BootstrapPayload::InitOk,
                            },
                        })
                        .expect("failed to send init ok message");
                    drop(bootstrap_tx);
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
            let message: Message<N::Payload> = serde_json::from_str(line.as_str())
                .expect("failed to deserialize line from input stream!");
            n.handle_message(message);
        }
    }
}
