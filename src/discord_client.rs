use crate::model::*;

use futures::{channel::mpsc::UnboundedSender, stream::StreamExt};
use futures_util::{future, pin_mut};
use regex::Regex;
use reqwest::header::HeaderMap;
use reqwest::Client;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time;

const PEPE_ID: &str = "270904126974590976";

#[derive(Debug)]
struct DiscordMessage {
    pub data: MessageCreateData,
    pub http: SharedHttpClient,
    pub client: SharedDiscordClient,
}

impl DiscordMessage {
    fn is_pepe_bot(&self) -> bool {
        return self.data.author.bot == Some(true) && self.data.author.id == PEPE_ID;
    }

    fn replied_to_me(&self, content: &str) -> bool {
        match &self.data.referenced_message {
            Some(ref_msg) => {
                ref_msg.author.id
                    == self
                        .client
                        .clone()
                        .lock()
                        .unwrap()
                        .user
                        .as_ref()
                        .unwrap()
                        .id
                    && ref_msg.content == content
            }
            None => false,
        }
    }

    async fn click_button(&self, row: usize, column: usize) {
        let button = &self.data.components[row].components[column];
        let client = self.client.lock().unwrap();
        if button.component_type == ComponentType::Button {
            let response = self
                .http
                .lock()
                .unwrap()
                .post("https://discord.com/api/v9/interactions")
                .body(
                    serde_json::to_string(&DiscordMessageInteraction {
                        session_id: client.session_id.to_string(),
                        application_id: self.data.author.id.to_string(),
                        channel_id: self.data.channel_id.to_string(),
                        discord_message_interaction_type: 3,
                        guild_id: self.data.guild_id.as_ref().unwrap().to_string(),
                        message_id: self.data.id.to_string(),
                        data: DiscordMessageInteractionComponent {
                            component_type: button.component_type,
                            custom_id: button.custom_id.to_string(),
                        },
                    })
                    .unwrap(),
                )
                .send()
                .await
                .unwrap();

            let json = &response.text().await.unwrap();
            println!("{}", json);
        }
    }

    async fn reply(&self, content: &str) -> DiscordMessage {
        let response = self
            .http
            .lock()
            .unwrap()
            .post(format!(
                "https://discord.com/api/v9/channels/{}/messages",
                self.data.channel_id
            ))
            .body(
                serde_json::to_string(&DiscordMessagePayload {
                    content: content.to_string(),
                    message_reference: Some(DiscordMessagePayloadReference {
                        channel_id: self.data.channel_id.to_string(),
                        guild_id: self.data.guild_id.as_ref().unwrap().to_string(),
                        message_id: self.data.id.to_string(),
                    }),
                })
                .unwrap(),
            )
            .send()
            .await
            .unwrap();
        return DiscordMessage {
            data: serde_json::from_str(&response.text().await.unwrap()).unwrap(),
            http: self.http.clone(),
            client: self.client.clone(),
        };
    }

    async fn send(&self, content: &str) -> DiscordMessage {
        let response = self
            .http
            .lock()
            .unwrap()
            .post(format!(
                "https://discord.com/api/v9/channels/{}/messages",
                self.data.channel_id
            ))
            .body(
                serde_json::to_string(&DiscordMessagePayload {
                    content: content.to_string(),
                    message_reference: None,
                })
                .unwrap(),
            )
            .send()
            .await
            .unwrap();
        return DiscordMessage {
            data: serde_json::from_str(&response.text().await.unwrap()).unwrap(),
            http: self.http.clone(),
            client: self.client.clone(),
        };
    }

    async fn edit(&self, content: &str) -> DiscordMessage {
        if self.data.author.id
            != self
                .client
                .clone()
                .lock()
                .unwrap()
                .user
                .as_ref()
                .unwrap()
                .id
        {
            panic!("Tried to edit a message that is not yours");
        }

        let response = self
            .http
            .lock()
            .unwrap()
            .patch(format!(
                "https://discord.com/api/v9/channels/{}/messages/{}",
                self.data.channel_id, self.data.id
            ))
            .body(
                serde_json::to_string(&DiscordMessagePayload {
                    content: content.to_string(),
                    message_reference: None,
                })
                .unwrap(),
            )
            .send()
            .await
            .unwrap();

        return DiscordMessage {
            data: serde_json::from_str(&response.text().await.unwrap()).unwrap(),
            http: self.http.clone(),
            client: self.client.clone(),
        };
    }

    fn on_updated(&self) -> DiscordMessage {
        todo!()
    }
}

#[derive(Debug)]
struct DiscordClient {
    transmitter: UnboundedSender<tokio_tungstenite::tungstenite::protocol::Message>,
    token: String,
    user: Option<ReadyDataUser>,
    sequence: u64,
    session_id: String,
}

struct Command<C, F>
where
    C: Fn(DiscordMessage) -> F,
    F: std::future::Future,
{
    command: String,
    cooldown: Duration,
    matcher: fn(&DiscordMessage) -> bool,
    execute: C,
}

type SharedDiscordClient = Arc<Mutex<DiscordClient>>;
type SharedHttpClient = Arc<Mutex<Client>>;

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
    let shared_client: SharedDiscordClient = Arc::new(Mutex::new(DiscordClient {
        transmitter,
        token: token.to_string(),
        sequence: 0,
        user: None,
        session_id: String::new(),
    }));

    // Discord HTTP API
    let mut headers = HeaderMap::new();
    headers.insert("Accept", "application/json".parse().unwrap());
    headers.insert("Content-Type", "application/json".parse().unwrap());
    headers.insert("Authorization", token.to_string().parse().unwrap());

    let http_client: SharedHttpClient = Arc::new(Mutex::new(
        Client::builder().default_headers(headers).build().unwrap(),
    ));

    // All Command Handlers
    let handlers = Arc::new(Mutex::new(vec![
        // High Low
        Command {
            command: "pls work".to_string(),
            cooldown: Duration::from_secs(30),
            matcher: |message| {
                return message.is_pepe_bot() && message.replied_to_me("pls hl");
            },
            execute: |message: DiscordMessage| async move {
                let description = message.data.embeds[0].description.as_ref().unwrap();
                let number_regex = Regex::new(r"\*\*(\d+)\*\*").unwrap();
                let number_string = number_regex.captures(description).unwrap();
                let number: u8 = number_string[1].parse().expect("not a number");
                message
                    .click_button(0, if number < 50 { 2 } else { 0 })
                    .await;
            },
        },
    ]));

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
                    token: get_token(&shared_client),
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
                    get_sequence(&shared_client),
                    None,
                )
                .await;
            }
            OpCode::Resume => {
                let data = ResumeData {
                    token: get_token(&shared_client),
                    session_id: get_session_id(&shared_client),
                    seq: get_sequence(&shared_client),
                };

                dispatch(shared_client.clone(), OpCode::Resume, data, None).await;
            }
            OpCode::HeartbeatAck => {}
            OpCode::Dispatch => {
                let event = package.t.unwrap();
                let data = package.d.unwrap();
                match event.as_str() {
                    "READY" => {
                        // write_to_json(&data, "ready.json");

                        let ready: ReadyData = serde_json::from_value(data).unwrap();
                        let arc = shared_client.clone();
                        let mut client = arc.lock().unwrap();
                        client.session_id = ready.session_id;
                        client.user = Some(ready.user);
                    }
                    "MESSAGE_CREATE" => {
                        // write_to_json(&data, "message_create.json");

                        let data: MessageCreateData = serde_json::from_value(data).unwrap();
                        let message: DiscordMessage = DiscordMessage {
                            data,
                            http: http_client.clone(),
                            client: shared_client.clone(),
                        };

                        let handlers_arc = handlers.clone();
                        let handlers = handlers_arc.lock().unwrap();
                        let handler = handlers.iter().find(|&h| (h.matcher)(&message));
                        match handler {
                            Some(handler) => {
                                (handler.execute)(message).await;
                            }
                            None => {
                                println!("Unhandled Message: {:?}", message.data.content);
                            }
                        }
                    }
                    "MESSAGE_UPDATE" => {
                        write_to_json(&data, "message_update.json");
                    }
                    "SESSION_REPLACE" => {}
                    "PRESENCE_UPDATE" => {}
                    _ => println!("Unhandled event: {}", event),
                }
            }
            _other => println!("Unhandled OpCode: {:?}", package.op),
        }
    });

    // Loop Through Commands
    // TODO: make looper

    pin_mut!(input_handler);
    future::select(ws_forwarding, input_handler).await;
}

fn write_to_json(data: &serde_json::Value, filename: &str) -> () {
    std::fs::write(format!("./data/{}", filename), data.to_string()).expect("File failed to write");
}

fn get_sequence(shared_client: &SharedDiscordClient) -> u64 {
    shared_client.clone().lock().unwrap().sequence
}

fn get_session_id(shared_client: &SharedDiscordClient) -> String {
    shared_client.clone().lock().unwrap().session_id.to_string()
}

fn get_token(shared_client: &SharedDiscordClient) -> String {
    shared_client.clone().lock().unwrap().token.to_string()
}

fn get_transmitter(
    shared_client: &SharedDiscordClient,
) -> UnboundedSender<tokio_tungstenite::tungstenite::protocol::Message> {
    shared_client.clone().lock().unwrap().transmitter.clone()
}

fn interval_heartbeat(shared_client: SharedDiscordClient, interval: u64) {
    let mut interval = time::interval(Duration::from_millis(interval));

    tokio::spawn(async move {
        interval.tick().await;
        loop {
            interval.tick().await;
            dispatch(
                shared_client.clone(),
                OpCode::Heartbeat,
                get_sequence(&shared_client),
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
        s: Some(get_sequence(&shared_client)),
    };

    get_transmitter(&shared_client)
        .unbounded_send(tokio_tungstenite::tungstenite::protocol::Message::text(
            &serde_json::to_string(&package).unwrap(),
        ))
        .unwrap();
}
