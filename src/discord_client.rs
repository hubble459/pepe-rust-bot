use crate::model::*;

use futures::{channel::mpsc::UnboundedSender, stream::StreamExt};
use futures_util::{future, pin_mut};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time;
use tokio_tungstenite::tungstenite::protocol::Message;

#[derive(Debug)]
struct DiscordClient {
    transmitter: UnboundedSender<Message>,
    token: String,
    sequence: u64,
    session_id: Option<String>,
}

type SharedClient = Arc<Mutex<DiscordClient>>;

pub async fn connect(token: String) {
    let (ws_stream, _) =
        tokio_tungstenite::connect_async("wss://gateway.discord.gg/?v=9&encoding=json")
            .await
            .expect("Failed to connect");
    println!("WebSocket handshake has been successfully completed");

    // Transmit and Receive for async Functions
    let (transmitter, receiver) = futures::channel::mpsc::unbounded();

    // Streams from WebSocket
    let (write, read) = ws_stream.split();

    // Forward all that is received
    let ws_forwarding = receiver.map(Ok).forward(write);

    // Discord Client
    let shared_client: SharedClient = Arc::new(Mutex::new(DiscordClient {
        transmitter: transmitter,
        token: token,
        sequence: 0,
        session_id: None,
    }));

    // Handler
    let input_handler = read.for_each(|msg| async {
        let json = msg.unwrap().into_text().unwrap();
        let package: Package = serde_json::from_str(&json).expect(&format!("Error: {:?}", json));
        match &package.s {
            Some(sequence) => shared_client.clone().lock().unwrap().sequence = *sequence,
            None => {}
        }
        println!(
            "{:?}. {:?}, has_data: {}",
            shared_client.clone().lock().unwrap().sequence,
            package.op,
            package.d.is_some()
        );
        match &package.op {
            OpCode::Hello => {
                let hello: HelloData = serde_json::from_value(package.d.unwrap()).unwrap();

                interval_heartbeat(shared_client.clone(), hello.heartbeat_interval);

                let data = IdentifyData {
                    token: get_token(shared_client.clone()),
                    properties: Properties {
                        browser: String::from("rust"),
                        device: String::from("rust"),
                        os: String::from(std::env::consts::OS),
                    },
                    compress: false,
                    presence: Presence {
                        activities: vec![Activity {
                            name: String::from("Dank Memer"),
                            activity_type: 0,
                        }],
                        status: String::from("online"),
                        since: 0,
                        afk: false,
                    },
                };
                // Login
                dispatch(shared_client.clone(), OpCode::Identify, &data, None).await;
            }
            OpCode::Heartbeat => {
                dispatch(
                    shared_client.clone(),
                    OpCode::Heartbeat,
                    get_sequence(shared_client.clone()),
                    None,
                )
                .await;
            }
            OpCode::Resume => {
                let data = ResumeData {
                    token: get_token(shared_client.clone()),
                    session_id: get_session_id(shared_client.clone()),
                    seq: get_sequence(shared_client.clone()),
                };

                dispatch(shared_client.clone(), OpCode::Resume, data, None).await;
            }
            OpCode::HeartbeatAck => {}
            OpCode::Dispatch => {
                let event = package.t.unwrap();
                let data = package.d.unwrap();
                match event.as_str() {
                    "READY" => {
                        let ready: ReadyData = serde_json::from_value(data).unwrap();
                        shared_client.clone().lock().unwrap().session_id = Some(ready.session_id);
                    }
                    "MESSAGE_CREATE" => {
                        std::fs::write("./owo.json", data.to_string())
                            .expect("File failed to write");

                        let message: MessageCreateData = serde_json::from_value(data).unwrap();

                        // TODO Make a message handler

                        println!("{:?}", message);
                    }
                    "MESSAGE_UPDATE" => {}
                    "SESSION_REPLACE" => {}
                    "PRESENCE_UPDATE" => {}
                    _ => println!("Unhandled event: {}", event),
                }
            }
            _other => println!("Unhandled OpCode: {:?}", package.op),
        }
    });

    pin_mut!(input_handler);
    future::select(ws_forwarding, input_handler).await;
}

fn get_sequence(shared_client: SharedClient) -> u64 {
    shared_client.clone().lock().unwrap().sequence
}

fn get_session_id(shared_client: SharedClient) -> String {
    shared_client
        .clone()
        .lock()
        .unwrap()
        .session_id
        .as_ref()
        .unwrap()
        .to_string()
}

fn get_token(shared_client: SharedClient) -> String {
    shared_client.clone().lock().unwrap().token.to_string()
}

fn get_transmitter(shared_client: SharedClient) -> UnboundedSender<Message> {
    shared_client.clone().lock().unwrap().transmitter.clone()
}

fn interval_heartbeat(shared_client: SharedClient, interval: u64) {
    let mut interval = time::interval(Duration::from_millis(interval));

    tokio::spawn(async move {
        interval.tick().await;
        loop {
            interval.tick().await;
            dispatch(
                shared_client.clone(),
                OpCode::Heartbeat,
                get_sequence(shared_client.clone()),
                None,
            )
            .await;
        }
    });
}

async fn dispatch<'a, T>(
    shared_client: SharedClient,
    op_code: OpCode,
    data: T,
    event: Option<&'a str>,
) where
    T: Serialize,
{
    let package = PackageWithData::<T> {
        op: op_code,
        d: data,
        t: match event {
            Some(evt) => Some(evt.to_string()),
            _ => None,
        },
        s: Some(get_sequence(shared_client.clone())),
    };

    get_transmitter(shared_client.clone())
        .unbounded_send(Message::text(&serde_json::to_string(&package).unwrap()))
        .unwrap();
}
