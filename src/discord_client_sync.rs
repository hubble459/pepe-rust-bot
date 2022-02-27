extern crate futures;

use crate::discord_commands::get_commands;
use crate::discord_message::*;
use crate::model::*;

use futures::lock::Mutex;
use futures::SinkExt;
use futures::StreamExt;
use reqwest::header::HeaderMap;
use reqwest::Client;
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::{stream::MaybeTlsStream, WebSocket};

struct Command {
    command: String,
    cooldown: Duration,
    last_called: Option<Instant>,
    matcher: fn(&DiscordMessage) -> bool,
    execute: fn(&DiscordMessage) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>,
}

fn make_http_client(token: &String) -> Client {
    let mut headers = HeaderMap::new();
    headers.insert("Accept", "application/json".parse().unwrap());
    headers.insert("Content-Type", "application/json".parse().unwrap());
    headers.insert("Authorization", token.to_string().parse().unwrap());
    Client::builder().default_headers(headers).build().unwrap()
}

pub async fn connect(token: String, master_id: String, channel_id: String) {
    let (stream, _) = tokio_tungstenite::connect_async_tls_with_config(
        "wss://gateway.discord.gg/?v=9&encoding=json",
        None,
        None,
    )
    .await
    .expect("Failed to connect with the Discord Gateway");
    let (sink, stream) = stream.split();
    let (message_sender, message_receiver) = crossbeam_channel::unbounded::<DiscordMessage>();

    let discord_client = DiscordClient {
        http: make_http_client(&token),
        master_id,
        session_id: "".to_owned(),
        token,
        user: None,
        sequence: 0,
        message_sender: message_sender.clone(),
        message_receiver: message_receiver.clone(),
        websocket_writer: sink,
    };

    let shared_client: SharedDiscordClient = Arc::new(Mutex::new(discord_client));

    let command_handler = tokio::spawn(async move {
        let receiver = message_receiver.clone();
        for message in receiver {
            let commands = get_commands();

            if message.data.edited_timestamp.is_some() {
                // Message is edited
                println!("Got Update");
                return;
            }

            let handler = commands.iter().find(|handler| (handler.matcher)(&message));
            match handler {
                Some(handler) => {
                    let h = handler.clone();
                    println!("Command Found: {:#?}", &h.command);
                    let result = (h.execute)(&message).await;
                    match result {
                        Ok(()) => {}
                        Err(error) => println!("Error in Command {:#?}", error),
                    }
                }
                None => {}
            }
        }
    });

    let shared_client_clone = shared_client.clone();

    let message_handler = stream.for_each_concurrent(None, |result| async {
        match result {
            Ok(message) => match &message {
                Message::Text(json) => {
                    let package: Result<Package, serde_json::Error> = serde_json::from_str(json);
                    match package {
                        Ok(package) => {
                            let scc = shared_client_clone.clone();
                            tokio::spawn(async move {
                                handle_ws_package(scc.clone(), package).await;
                            });
                        }
                        Err(error) => {
                            println!("Not JSON: {:#?}", json);
                            println!("Error: {:#?}", error);
                        }
                    }
                }
                _else => {
                    println!("Received Unknown Message: {:#?}", message);
                }
            },
            Err(error) => {
                println!("Error Occurred: {:#?}", error);
            }
        }
    });

    futures::future::select(message_handler, command_handler).await;
    // message_handler.await.unwrap();
}

/// Handles a Discord WebSocket Package
async fn handle_ws_package(shared_client: SharedDiscordClient, package: Package) {
    // Set the sequence if there is one in the package
    match &package.sequence {
        Some(sequence) => shared_client.clone().lock().await.sequence = *sequence,
        None => {}
    }

    // Log the Package
    println!(
        "{:?}. {:?}, has_data: {}",
        shared_client.clone().lock().await.sequence,
        package.op,
        package.data.is_some()
    );

    match &package.op {
        OpCode::Hello => {
            let hello: HelloData = serde_json::from_value(package.data.unwrap()).unwrap();

            interval_heartbeat(shared_client.clone(), hello.heartbeat_interval);

            let data = IdentifyData {
                token: get_token(&shared_client).await,
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
                get_sequence(&shared_client).await,
                None,
            )
            .await;
        }
        OpCode::Resume => {
            let client = shared_client.clone();
            let client = client.lock().await;

            let data = ResumeData {
                token: client.token.clone(),
                session_id: client.session_id.clone(),
                seq: client.sequence,
            };

            dispatch(shared_client.clone(), OpCode::Resume, data, None).await;
        }
        OpCode::HeartbeatAck => {}
        OpCode::Dispatch => {
            let event = package.tag.unwrap();
            let data = package.data.unwrap();
            match event.as_str() {
                "READY" => {
                    // write_to_json(&data, "ready.json");

                    let ready: ReadyData = serde_json::from_value(data).unwrap();
                    let arc = shared_client.clone();
                    let mut client = arc.lock().await;
                    client.session_id = ready.session_id;
                    client.user = Some(ready.user);
                }
                "MESSAGE_CREATE" => {
                    // write_to_json(&data, "message_create.json");

                    let c_client = shared_client.clone();
                    let client = c_client.lock().await;
                    let data: MessageCreateData = serde_json::from_value(data).unwrap();
                    let message: DiscordMessage = DiscordMessage {
                        data,
                        user: client.user.as_ref().unwrap().clone(),
                        client: shared_client.clone(),
                    };

                    client.message_sender.clone().send(message).unwrap();
                }
                "MESSAGE_UPDATE" => {
                    println!("omg update: before client");

                    let c_client = shared_client.clone();
                    let client = c_client.lock().await;
                    let data: MessageCreateData = serde_json::from_value(data).unwrap();
                    let message: DiscordMessage = DiscordMessage {
                        data,
                        user: client.user.as_ref().unwrap().clone(),
                        client: shared_client.clone(),
                    };

                    println!("omg update: after client");
                    client.message_sender.clone().send(message).unwrap();
                }
                "SESSION_REPLACE" => {}
                "PRESENCE_UPDATE" => {}
                "SESSIONS_REPLACE" => {}
                _ => println!("Unhandled event: {}", event),
            }
        }
        _other => println!("Unhandled OpCode: {:?}", package.op),
    }
}

async fn get_sequence(shared_client: &SharedDiscordClient) -> u64 {
    shared_client.clone().lock().await.sequence
}

async fn get_session_id(shared_client: &SharedDiscordClient) -> String {
    shared_client.clone().lock().await.session_id.to_string()
}

async fn get_token(shared_client: &SharedDiscordClient) -> String {
    shared_client.clone().lock().await.token.to_string()
}

fn interval_heartbeat(shared_client: SharedDiscordClient, heartbeat_interval: u64) {
    let mut interval = tokio::time::interval(Duration::from_millis(heartbeat_interval));

    let client = shared_client.clone();

    tokio::spawn(async move {
        interval.tick().await;

        loop {
            interval.tick().await;

            dispatch(
                client.clone(),
                OpCode::Heartbeat,
                get_sequence(&shared_client).await,
                None,
            )
            .await;
        }
    });
}

async fn dispatch<'a, T>(
    shared_client: SharedDiscordClient,
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
            None => None,
        },
        s: Some(get_sequence(&shared_client).await),
    };
    shared_client
        .clone()
        .lock()
        .await
        .websocket_writer
        .send(Message::text(&serde_json::to_string(&package).unwrap()))
        .await
        .unwrap();
}
