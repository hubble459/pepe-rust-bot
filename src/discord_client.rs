extern crate futures;

use crate::discord_commands;
use crate::discord_commands::Command;
use crate::discord_message::*;
use crate::model::*;

use async_recursion::async_recursion;
use futures::lock::Mutex;
use futures::SinkExt;
use futures::StreamExt;
use log::warn;
use rand::Rng;
use reqwest::header::HeaderMap;
use reqwest::Client;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;

use log::{debug, error, info};

fn make_http_client(token: &String) -> Client {
    let mut headers = HeaderMap::new();
    headers.insert("Accept", "application/json".parse().unwrap());
    headers.insert("Content-Type", "application/json".parse().unwrap());
    headers.insert("Authorization", token.to_string().parse().unwrap());
    Client::builder().default_headers(headers).build().unwrap()
}

#[async_recursion]
pub async fn connect(token: String, master_id: Option<String>, channel_id: Option<String>) {
    let (stream, _) = tokio_tungstenite::connect_async_tls_with_config(
        "wss://gateway.discord.gg/?v=9&encoding=json",
        None,
        None,
    )
    .await
    .expect("Failed to connect with the Discord Gateway");

    info!("Connected to the Discord WebSocket");

    if master_id.is_none() {
        warn!("Does not have a master!");
        warn!("Will only listen to self");
    }

    let (sink, stream) = stream.split();
    let (message_update_sender, message_update_receiver) =
        async_channel::unbounded::<DiscordMessage>();
    let (master_command_sender, master_command_receiver) =
        async_channel::unbounded::<MasterCommand>();

    let shared_client: SharedDiscordClient = Arc::new(Mutex::new(DiscordClient {
        http: make_http_client(&token),
        master_id: master_id.clone(),
        session_id: "".to_owned(),
        token: token.to_string(),
        user: None,
        sequence: 0,
        websocket_writer: sink,
        message_update_receiver,
        master_command_sender: master_command_sender.clone(),
        commands: discord_commands::get_commands(),
    }));
    let shared_client_clone = shared_client.clone();

    let shared_channel_id: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(channel_id.clone()));
    let shared_channel_id_clone = shared_channel_id.clone();

    let command_loop = tokio::spawn(async move {
        let mut tokio_thread: Option<JoinHandle<()>> = None;

        info!("Listening for Master Commands");

        loop {
            let master_command = master_command_receiver.recv().await.unwrap();
            let is_running = &tokio_thread.is_some() == &true;

            match master_command.command {
                MasterCommandType::Start => {
                    let channel_id = master_command.tag.unwrap().to_string();
                    *shared_channel_id_clone.clone().lock().await = Some(channel_id.to_string());
                    let mut interval = tokio::time::interval(Duration::from_secs(1));
                    let shared_client_1 = shared_client.clone();
                    let shared_client_2 = shared_client.clone();
                    let cmd_client = shared_client_1.lock().await;
                    let mut runnable_commands = cmd_client
                        .commands
                        .clone()
                        .into_iter()
                        .filter(|c| c.command.is_some())
                        .collect::<Vec<Command>>();
                    let commands_length = runnable_commands.len();

                    if !is_running {
                        info!("Running in {}", channel_id.to_string());
                        tokio_thread = Some(tokio::spawn(async move {
                            interval.tick().await;
                            loop {
                                interval.tick().await;
                                let mut command = &mut runnable_commands
                                    [rand::thread_rng().gen_range(0..commands_length)];
                                if command.last_called.is_none()
                                    || command.last_called.unwrap().elapsed() >= command.cooldown
                                {
                                    command.last_called = Some(Instant::now());
                                    let command_content =
                                        command.command.as_ref().unwrap().to_string();
                                    shared_client_2
                                        .clone()
                                        .lock()
                                        .await
                                        .http
                                        .post(format!(
                                            "https://discord.com/api/v9/channels/{}/messages",
                                            channel_id.to_string()
                                        ))
                                        .body(
                                            serde_json::to_string(&DiscordMessagePayload {
                                                content: command_content,
                                                message_reference: None,
                                            })
                                            .unwrap(),
                                        )
                                        .send()
                                        .await
                                        .unwrap();
                                }
                            }
                        }));
                    }
                }
                MasterCommandType::Stop => {
                    if is_running {
                        tokio_thread.as_ref().unwrap().abort();
                        tokio_thread = None;
                    }
                }
            }
        }
    });

    let message_handler = stream.for_each(|result| async {
        let mus = message_update_sender.clone();
        match result {
            Ok(message) => match &message {
                Message::Text(json) => {
                    let package: Result<Package, serde_json::Error> = serde_json::from_str(json);
                    match package {
                        Ok(package) => {
                            let scc = shared_client_clone.clone();
                            tokio::spawn(async move {
                                handle_ws_package(scc.clone(), package, mus).await;
                            });
                        }
                        Err(error) => {
                            error!("Not JSON: {:#?}", json);
                            error!("Error: {:#?}", error);
                        }
                    }
                }
                _else => {
                    error!("Received Unknown Message: {:#?}", message);
                }
            },
            Err(error) => {
                error!("Error Occurred: {:#?}", error);
            }
        }
    });

    // If channel id is known, start on this channel
    if channel_id.is_some() {
        master_command_sender
            .clone()
            .send(MasterCommand {
                command: MasterCommandType::Start,
                tag: Some(channel_id.unwrap()),
            })
            .await
            .unwrap();
    }

    // pin_mut!(message_handler);
    // futures::future::select(message_handler, command_loop);

    message_handler.await;

    info!("Disconnected from the Discord Gateway");
    info!("Closing threads");
    command_loop.abort();

    info!("Trying to reconnect...");

    // Disconnected so try to reconnect
    connect(
        token.to_owned(),
        master_id.to_owned(),
        shared_channel_id.lock().await.clone(),
    )
    .await;
}

/// Handles a Discord WebSocket Package
async fn handle_ws_package(
    shared_client: SharedDiscordClient,
    package: Package,
    message_update_sender: async_channel::Sender<DiscordMessage>,
) {
    // Set the sequence if there is one in the package
    match &package.sequence {
        Some(sequence) => shared_client.clone().lock().await.sequence = *sequence,
        None => {}
    }

    // Log the Package
    debug!(
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
        OpCode::Reconnect => {
            debug!("Reconnect: {:#?}", package);

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
            let json_data = package.data.unwrap();
            match event.as_str() {
                "READY" => {
                    // std::fs::write("data/ready.json", json_data.to_string()).expect("Unable to write file");

                    let ready: ReadyData = serde_json::from_value(json_data).unwrap();

                    info!("Logged in as {}", &ready.user.username);

                    let arc = shared_client.clone();
                    let mut client = arc.lock().await;
                    client.session_id = ready.session_id;
                    client.user = Some(ready.user);
                }
                "MESSAGE_CREATE" => {
                    // std::fs::write("data/message_create.json", json_data.to_string()).expect("Unable to write file");

                    let data = serde_json::from_value::<MessageCreateData>(json_data);
                    match data {
                        Ok(data) => {
                            let message: DiscordMessage =
                                DiscordMessage::new(data, shared_client.clone()).await;
                            let client = shared_client.clone();
                            let client = client.lock().await;

                            let commands = client.commands.clone();
                            drop(client);

                            // Get command handler
                            let handlers = commands
                                .into_iter()
                                .filter(|handler| (handler.matcher)(&message));
                            for handler in handlers {
                                let result = (handler.execute)(&message).await;
                                match result {
                                    Ok(()) => {}
                                    Err(error) => error!("Error in Command {:#?}", error),
                                }
                            }
                        }
                        Err(error) => {
                            error!("Error: {:#?}", error);
                        }
                    }
                }
                "MESSAGE_UPDATE" => {
                    let data: MessageCreateData = serde_json::from_value(json_data).unwrap();
                    let message: DiscordMessage = DiscordMessage::new(data, shared_client).await;

                    message_update_sender.clone().send(message).await.unwrap();
                }
                "SESSION_REPLACE" => {}
                "PRESENCE_UPDATE" => {}
                "SESSIONS_REPLACE" => {}
                "INTERACTION_CREATE" => {}
                "INTERACTION_SUCCESS" => {}
                "MESSAGE_ACK" => {}
                _ => debug!("Unhandled event: {}", event),
            }
        }
        _other => debug!("Unhandled OpCode: {:?}", package.op),
    }
}

async fn get_sequence(shared_client: &SharedDiscordClient) -> u64 {
    shared_client.clone().lock().await.sequence
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
