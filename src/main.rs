use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    stream::StreamExt,
};
use futures_util::{future, pin_mut};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{task::yield_now, time};
use tokio_tungstenite::tungstenite::protocol::Message;
// use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Deserialize, Serialize, PartialEq, Debug)]
struct HelloData {
    heartbeat_interval: u64,
}

#[derive(Deserialize, Serialize, PartialEq, Debug)]
struct Package<T> {
    op: OpCode,
    d: T,
}

#[derive(Deserialize, Serialize, PartialEq, Debug)]
struct PackageOpCode {
    op: OpCode,
}

#[derive(Deserialize_repr, Serialize_repr, PartialEq, Debug)]
#[repr(u8)]
enum OpCode {
    Dispatch = 0,
    Heartbeat = 1,
    Identify = 2,
    PresenceUpdate = 3,
    VoiceStateUpdate = 4,
    Resume = 6,
    Reconnect = 7,
    RequestGuildMembers = 8,
    InvalidSession = 9,
    Hello = 10,
    HeartbeatAck = 11,
}

#[derive(Deserialize, Serialize, PartialEq, Debug)]
struct Heartbeat {
    op: OpCode,
    d: u64,
}

#[derive(Deserialize, Serialize, PartialEq, Debug)]
struct Payload<'a, T> {
    op: OpCode,
    s: u64,
    d: T,
    t: &'a str,
}

impl<T> Payload<'_, T> {
    pub fn new(op_code: OpCode, event: &str, sequence: u64, data: T) -> Payload<'_, T> {
        Payload {
            op: op_code,
            d: data,
            s: sequence,
            t: event,
        }
    }
}

#[derive(Debug)]
struct DiscordClient {
    sequence: u64,
    token: String,
}

type Counter = Arc<Mutex<u64>>;
type SharedTransmitter = Arc<Mutex<UnboundedSender<Message>>>;

impl DiscordClient {
    pub async fn connect(&mut self) {
        let (ws_stream, _) =
            tokio_tungstenite::connect_async("wss://gateway.discord.gg/?v=9&encoding=json")
                .await
                .expect("Failed to connect");
        println!("WebSocket handshake has been successfully completed");

        // Transmit and Receive for async Functions
        let (transmitter, receiver) = futures::channel::mpsc::unbounded();

        let tx: SharedTransmitter = Arc::new(Mutex::new(transmitter));
        let sq: Counter = Arc::new(Mutex::new(0));

        // Streams from WebSocket
        let (write, read) = ws_stream.split();
        // Forward all that is received
        let ws_forwarding = receiver.map(Ok).forward(write);
        // Handler
        let input_handler = read.for_each(|msg| async {
            let tx = tx.clone();
            let sq = sq.clone();

            let json = msg.unwrap().into_text().unwrap();
            println!("{:?}", json);
            let op_code_pkg: PackageOpCode =
                serde_json::from_str(&json).expect(&format!("Error: {:?}", json));
            let op = op_code_pkg.op;
            match op {
                OpCode::Hello => {
                    let hello: Package<HelloData> = serde_json::from_str(&json).unwrap();
                    tokio::spawn(interval_heartbeat(tx, sq, hello.d.heartbeat_interval));
                }
                OpCode::Heartbeat => {
                    let sq2 = sq.clone();
                    heartbeat(tx, sq).await;
                    let mut guard = sq2.lock().unwrap();
                    *guard += 1;
                }
                _other => println!(
                    "Unhandled OpCode: {:?}",
                    serde_json::from_str::<PackageOpCode>(&json)
                        .expect(&format!("Error: {:?}", json))
                        .op
                ),
            }
        });

        pin_mut!(input_handler);
        future::select(ws_forwarding, input_handler).await;
        // input_handler.await;
    }

    // fn dispatch<'a, T>(&self, event: &'a str, data: T) -> Payload<'a, T> {
    //     let payload = Payload::new(OpCode::Dispatch, event, self.sequence, data);
    //     self.sequence += 1;

    //     return payload;
    // }

    fn new<'a>(token: &'a str) -> DiscordClient {
        DiscordClient {
            token: token.to_string(),
            sequence: 0,
        }
    }
}

async fn interval_heartbeat(tx: SharedTransmitter, sq: Counter, interval: u64) {
    let mut interval = time::interval(Duration::from_millis(interval));

    tokio::spawn(async move {
        interval.tick().await;
        loop {
            let tx = tx.clone();
            let sq = sq.clone();
            let sq2 = sq.clone();
            interval.tick().await;
            heartbeat(tx, sq).await;

            {
                let mut guard = sq2.lock().unwrap();
                *guard += 1;
            }

            yield_now().await;
        }
    });
}

async fn heartbeat(tx: SharedTransmitter, sq: Counter) {
    tx.lock()
        .unwrap()
        .unbounded_send(Message::text(
            &serde_json::to_string(&Heartbeat {
                op: OpCode::Heartbeat,
                d: *sq.lock().unwrap(),
            })
            .unwrap(),
        ))
        .unwrap();
}

#[tokio::main]
async fn main() {
    let mut client = DiscordClient::new("owo");
    client.connect().await;
}
