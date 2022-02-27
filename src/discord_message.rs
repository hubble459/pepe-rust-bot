use futures::lock::Mutex;
use std::sync::Arc;

use reqwest::Client;

use crate::model::*;

const PEPE_ID: &str = "270904126974590976";

pub struct DiscordClient {
    pub token: String,
    pub user: Option<ReadyDataUser>,
    pub sequence: u64,
    pub session_id: String,
    pub master_id: String,
    // pub message_receiver: std::sync::mpsc::Receiver<DiscordMessage>,
    pub http: Client,
    pub websocket_writer: futures::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        tokio_tungstenite::tungstenite::Message,
    >,
    pub message_receiver: crossbeam_channel::Receiver<DiscordMessage>,
    pub message_sender: crossbeam_channel::Sender<DiscordMessage>,
}

pub type SharedDiscordClient = Arc<Mutex<DiscordClient>>;

pub struct DiscordMessage {
    pub user: ReadyDataUser,
    pub data: MessageCreateData,
    pub client: SharedDiscordClient,
}

impl DiscordMessage {
    pub fn is_pepe_bot(&self) -> bool {
        return self.data.author.bot == Some(true) && self.data.author.id == PEPE_ID;
    }

    pub fn replied_to_me(&self, content: &str) -> bool {
        match &self.data.referenced_message {
            Some(ref_msg) => ref_msg.author.id == self.user.id && ref_msg.content == content,
            None => false,
        }
    }

    pub fn embed_title_contains(&self, content: &str) -> bool {
        return !self.data.embeds.is_empty()
            && self.data.embeds[0].title.is_some()
            && self.data.embeds[0]
                .title
                .as_ref()
                .unwrap()
                .to_lowercase()
                .contains(&content.to_lowercase());
    }

    pub fn embed_author_contains(&self, content: &str) -> bool {
        return !self.data.embeds.is_empty()
            && self.data.embeds[0].author.is_some()
            && self.data.embeds[0]
                .author
                .as_ref()
                .unwrap()
                .name
                .to_lowercase()
                .contains(&content.to_lowercase());
    }

    pub fn embed_description_contains(&self, content: &str) -> bool {
        return !self.data.embeds.is_empty()
            && self.data.embeds[0].description.is_some()
            && self.data.embeds[0]
                .description
                .as_ref()
                .unwrap()
                .to_lowercase()
                .contains(&content.to_lowercase());
    }

    pub async fn click_button(&self, row: usize, column: usize) {
        if row >= self.data.components.len() {
            return;
        }
        let row = &self.data.components[row];
        if column >= row.components.len() {
            return;
        }
        let button = &row.components[column];
        if button.component_type == ComponentType::Button
            && (button.disabled.is_none() || button.disabled == Some(false))
        {
            let client_c = self.client.clone();
            let client = client_c.lock().await;
            let body = serde_json::to_string(&DiscordMessageInteraction {
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
            .unwrap();
            let http = client.http.clone();
            drop(client);

            http.post("https://discord.com/api/v9/interactions")
                .body(body)
                .send()
                .await
                .unwrap();
        }
    }

    pub async fn reply(&self, content: &str) -> DiscordMessage {
        let response = self
            .client
            .clone()
            .lock()
            .await
            .http
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
            user: self.user.clone(),
            data: serde_json::from_str(&response.text().await.unwrap()).unwrap(),
            client: self.client.clone(),
        };
    }

    pub async fn send(&self, content: &str) -> DiscordMessage {
        let response = self
            .client
            .clone()
            .lock()
            .await
            .http
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
            user: self.user.clone(),
            data: serde_json::from_str(&response.text().await.unwrap()).unwrap(),
            client: self.client.clone(),
        };
    }

    pub async fn edit(&self, content: &str) -> DiscordMessage {
        if self.data.author.id != self.client.clone().lock().await.user.as_ref().unwrap().id {
            panic!("Tried to edit a message that is not yours");
        }

        let response = self
            .client
            .clone()
            .lock()
            .await
            .http
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
            user: self.user.clone(),
            data: serde_json::from_str(&response.text().await.unwrap()).unwrap(),
            client: self.client.clone(),
        };
    }

    pub async fn wait_update(&self) -> DiscordMessage {
        loop {
            let client_c = self.client.clone();
            let client = client_c.lock().await;
            println!("wu: got client");
            let receiver = client.message_receiver.clone();
            println!("wu: got receiver");
            let message = receiver.recv();
            match message {
                Ok(message) => {
                    println!("got {}", message.data.id);

                    if message.data.id == self.data.id {
                        break message;
                    }
                }
                Err(e) => {
                    println!("got error: {:#?}", e);
                }
            }
        }
    }
}
