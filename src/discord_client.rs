use crate::model::*;

use futures::future::Future;
use futures::{channel::mpsc::UnboundedSender, stream::StreamExt};
use futures_util::{future, pin_mut};
use rand::Rng;
use regex::Regex;
use reqwest::header::HeaderMap;
use reqwest::Client;
use serde::Serialize;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::task::yield_now;
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

    async fn wait_update(&self) -> DiscordMessage {
        println!("waiting");
        loop {
            let message = self
                .client
                .clone()
                .lock()
                .unwrap()
                .dispatch_receiver
                .next()
                .await;

            match message {
                Some(msg) => {
                    println!("got {}", msg.data.id);

                    if msg.data.id == self.data.id {
                        break msg;
                    }
                }
                None => {
                    println!("got None");
                }
            }
        }
    }
}

#[derive(Debug)]
struct DiscordClient {
    transmitter: UnboundedSender<tokio_tungstenite::tungstenite::protocol::Message>,
    token: String,
    user: Option<ReadyDataUser>,
    sequence: u64,
    session_id: String,
    master_id: String,
    dispatch_receiver: futures::channel::mpsc::UnboundedReceiver<DiscordMessage>,
}

struct Command {
    command: String,
    cooldown: Duration,
    last_called: Option<Instant>,
    matcher: fn(&DiscordMessage) -> bool,
    execute: Box<
        dyn Fn(DiscordMessage) -> Pin<Box<dyn Future<Output = ()>>> + Sync + Send,
    >,
}

type SharedDiscordClient = Arc<Mutex<DiscordClient>>;
type SharedHttpClient = Arc<Mutex<Client>>;

fn make_http_client(token: &String) -> Client {
    let mut headers = HeaderMap::new();
    headers.insert("Accept", "application/json".parse().unwrap());
    headers.insert("Content-Type", "application/json".parse().unwrap());
    headers.insert("Authorization", token.to_string().parse().unwrap());
    Client::builder().default_headers(headers).build().unwrap()
}

pub async fn connect(token: String, master_id: String, channel_id: String) {
    let (ws_stream, _) =
        tokio_tungstenite::connect_async("wss://gateway.discord.gg/?v=9&encoding=json")
            .await
            .expect("Failed to connect");
    println!("WebSocket handshake has been successfully completed");

    // Transmit and Receive for async Functions
    let (transmitter, receiver) = futures::channel::mpsc::unbounded();
    let (dispatch_sender, dispatch_receiver) =
        futures::channel::mpsc::unbounded::<DiscordMessage>();

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
        master_id,
        dispatch_receiver,
    }));

    // Discord HTTP API
    let http_client: SharedHttpClient = Arc::new(Mutex::new(make_http_client(&token)));

    // All Command Handlers
    let commands = vec![
        // High Low
        // Command {
        //     last_called: None,
        //     command: String::from("pls hl"),
        //     cooldown: Duration::from_secs(30),
        //     matcher: |message| message.is_pepe_bot() && message.replied_to_me("pls hl"),
        //     execute: Box::new(|message: DiscordMessage| {
        //         Box::pin(async move {
        //             let description = message.data.embeds[0].description.as_ref().unwrap();
        //             let number_regex = Regex::new(r"\*\*(\d+)\*\*").unwrap();
        //             let number_string = number_regex.captures(description).unwrap();
        //             let number: u8 = number_string[1].parse().expect("not a number");
        //             message
        //                 .click_button(0, if number < 50 { 2 } else { 0 })
        //                 .await;
        //         })
        //     }),
        // },
        // // Hunting
        // Command {
        //     last_called: None,
        //     command: String::from("pls hunt"),
        //     cooldown: Duration::from_secs(40),
        //     matcher: |message| {
        //         message.is_pepe_bot()
        //             && message.replied_to_me("pls hunt")
        //             && message.data.content.starts_with("Dodge the Fireball")
        //     },
        //     execute: Box::new(|message: DiscordMessage| {
        //         Box::pin(async move {
        //             let content = &message.data.content;
        //             let fireball_line: &str = content.split("\n").collect::<Vec<&str>>()[2];
        //             message
        //                 .click_button(
        //                     0,
        //                     if fireball_line.starts_with("       ") {
        //                         0
        //                     } else {
        //                         1
        //                     },
        //                 )
        //                 .await;
        //         })
        //     }),
        // },
        // // Fishing
        // Command {
        //     last_called: None,
        //     command: String::from("pls fish"),
        //     cooldown: Duration::from_secs(40),
        //     matcher: |message| {
        //         message.is_pepe_bot()
        //             && message.replied_to_me("pls fish")
        //             && message.data.content.starts_with("Catch the fish!")
        //     },
        //     execute: Box::new(|message: DiscordMessage| {
        //         Box::pin(async move {
        //             let content = &message.data.content;
        //             let fireball_line: &str = content.split("\n").collect::<Vec<&str>>()[1];
        //             message
        //                 .click_button(
        //                     0,
        //                     if fireball_line.starts_with("              ") {
        //                         2
        //                     } else if fireball_line.starts_with("       ") {
        //                         1
        //                     } else {
        //                         0
        //                     },
        //                 )
        //                 .await;
        //         })
        //     }),
        // },
        // // Digging
        // Command {
        //     last_called: None,
        //     command: String::from("pls dig"),
        //     cooldown: Duration::from_secs(40),
        //     matcher: |_message| false,
        //     execute: Box::new(|_message: DiscordMessage| Box::pin(async move {})),
        // },
        // // Begging
        // Command {
        //     last_called: None,
        //     command: String::from("pls beg"),
        //     cooldown: Duration::from_secs(45),
        //     matcher: |_message| false,
        //     execute: Box::new(|_message: DiscordMessage| Box::pin(async move {})),
        // },
        // // Deposit All
        // Command {
        //     last_called: None,
        //     command: String::from("pls dep all"),
        //     cooldown: Duration::from_secs(60),
        //     matcher: |_message| false,
        //     execute: Box::new(|_message: DiscordMessage| Box::pin(async move {})),
        // },
        // Post Memes
        Command {
            last_called: None,
            command: String::from("pls pm"),
            cooldown: Duration::from_secs(30),
            matcher: |message| message.is_pepe_bot() && message.replied_to_me("pls pm"),
            execute: Box::new(|message: DiscordMessage| {
                Box::pin(async move {
                    {
                        message
                            .click_button(0, rand::thread_rng().gen_range(0..5))
                            .await;
                        let updated = message.wait_update().await;
                        if updated.data.embeds[0]
                            .description
                            .as_ref()
                            .unwrap()
                            .contains("**Laptop** is broken")
                        {
                            updated.send("pls buy laptop").await;
                        }
                    }
                })
            }),
        },
        // Work
        // Command {
        //     last_called: None,
        //     command: String::from("pls work"),
        //     cooldown: Duration::from_secs(3600),
        //     matcher: |_message| false,
        //     execute: Box::new(|_message: DiscordMessage| Box::pin(async move {})),
        // },
        // Pet
        // Command {
        //     last_called: None,
        //     command: String::from("pls pet"),
        //     cooldown: Duration::from_secs(3600),
        //     matcher: |_message| false,
        //     execute: Box::new(|_message: DiscordMessage| Box::pin(async move {})),
        // },
        // Daily
        // Command {
        //     last_called: None,
        //     command: String::from("pls daily"),
        //     cooldown: Duration::from_secs(3600),
        //     matcher: |_message| false,
        //     execute: Box::new(|_message: DiscordMessage| Box::pin(async move {})),
        // },
    ];
    // let handlers_arc = Arc::new(commands);
    let handlers = Arc::new(Mutex::new(commands));

    loop_commands(token.to_string(), channel_id, handlers.clone());

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
                        let handlers_vec = handlers_arc.lock().unwrap();
                        let handler = handlers_vec.iter().find(|&h| (h.matcher)(&message));
                        match handler {
                            Some(handler) => {
                                // std::thread::spawn(move || async {
                                    (handler.execute)(message).await;
                                // });
                            }
                            None => {
                                // Ignored
                            }
                        }
                    }
                    "MESSAGE_UPDATE" => {
                        let data: MessageCreateData = serde_json::from_value(data).unwrap();
                        let message: DiscordMessage = DiscordMessage {
                            data,
                            http: http_client.clone(),
                            client: shared_client.clone(),
                        };
                        println!("dispatching");
                        dispatch_sender.unbounded_send(message).unwrap();
                    }
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

fn loop_commands(token: String, channel_id: String, handlers: Arc<Mutex<Vec<Command>>>) {
    let mut interval = time::interval(Duration::from_millis(2000));

    tokio::spawn(async move {
        interval.tick().await;

        let http = make_http_client(&token);

        // Loop Through Commands
        loop {
            interval.tick().await;

            let mut command_string: Vec<String> = vec![];
            let arc = handlers.clone();
            {
                let mut handlers = arc.lock().unwrap();
                let length = handlers.len();
                let mut command = &mut handlers[rand::thread_rng().gen_range(0..length)];
                if command.last_called.is_some()
                    && command.last_called.unwrap().elapsed() < command.cooldown
                {
                    continue;
                } else {
                    command.last_called = Some(Instant::now());
                    command_string.push(command.command.to_string());
                }
            }

            let cid = channel_id.to_string();
            let command = command_string[0].to_string();
            http.post(format!(
                "https://discord.com/api/v9/channels/{}/messages",
                cid.to_string()
            ))
            .body(
                serde_json::to_string(&DiscordMessagePayload {
                    content: command.to_string(),
                    message_reference: None,
                })
                .unwrap(),
            )
            .send()
            .await
            .unwrap();
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
