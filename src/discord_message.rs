use futures::lock::Mutex;
use std::{sync::Arc, time::Duration};
use tokio::time::timeout;

use reqwest::Client;

use crate::{model::*, custom_error::MyError};

const PEPE_ID: &str = "270904126974590976";

pub struct DiscordClient {
    pub token: String,
    pub user: Option<ReadyDataUser>,
    pub sequence: u64,
    pub session_id: String,
    pub master_id: String,
    pub http: Client,
    pub message_update_receiver: async_channel::Receiver<DiscordMessage>,
    pub websocket_writer: futures::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        tokio_tungstenite::tungstenite::Message,
    >,
    pub master_command_sender: async_channel::Sender<MasterCommand>,
    pub commands: Vec<crate::discord_commands::Command>,
}

pub type SharedDiscordClient = Arc<Mutex<DiscordClient>>;

pub struct DiscordMessage {
    pub master_id: String,
    pub user: ReadyDataUser,
    pub data: MessageCreateData,
    pub client: SharedDiscordClient,
}

impl DiscordMessage {
    pub async fn new(data: MessageCreateData, client: SharedDiscordClient) -> DiscordMessage {
        let client_arc = client.clone();
        let client_mutex = client_arc.lock().await;
        DiscordMessage {
            master_id: client_mutex.master_id.clone(),
            user: client_mutex.user.as_ref().unwrap().clone(),
            data,
            client,
        }
    }

    pub fn new_from(&self, data: MessageCreateData) -> DiscordMessage {
        DiscordMessage {
            data,
            master_id: self.master_id.clone(),
            user: self.user.clone(),
            client: self.client.clone(),
        }
    }

    pub fn replied_to_me(&self, content: &str) -> bool {
        match &self.data.referenced_message {
            Some(ref_msg) => ref_msg.author.id == self.user.id && ref_msg.content == content,
            None => false,
        }
    }

    pub fn is_from(&self, user_id: &str) -> bool {
        return self.data.author.id == user_id;
    }

    pub fn is_from_master(&self) -> bool {
        return self.data.author.id == self.master_id;
    }

    pub fn is_from_pepe(&self) -> bool {
        return self.is_from(PEPE_ID);
    }

    pub fn get_component(&self, row: usize, column: usize) -> Option<MessageComponent> {
        if row >= self.data.components.len() {
            return None;
        }
        let row = &self.data.components[row];
        if column >= row.components.len() {
            return None;
        }
        Some(row.components[column].clone())
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

    pub async fn click_button(
        &self,
        row: usize,
        column: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if row >= self.data.components.len() {
            return Err(Box::new(MyError::new("Component Row out of bounds")));
        }
        let row = &self.data.components[row];
        if column >= row.components.len() {
            return Err(Box::new(MyError::new("Component Button out of bounds")));
        }
        let button = &row.components[column];
        if button.component_type == ComponentType::Button && button.disabled == false {
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
                    custom_id: button.custom_id.as_ref().unwrap().to_string(),
                    type_type: None,
                    values: None,
                },
            })?;
            let http = client.http.clone();
            drop(client);

            http.post("https://discord.com/api/v9/interactions")
                .body(body)
                .send()
                .await?;
        }
        Ok(())
    }

    pub async fn select_option(
        &self,
        row: usize,
        option: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if row >= self.data.components.len() {
            return Err(Box::new(MyError::new("Component Row out of bounds")));
        }
        let row = &self.data.components[row];
        if row.components.is_empty() {
            return Err(Box::new(MyError::new("No Select Menu in Row")));
        }
        let select_menu = &row.components[0];
        if option >= select_menu.options.len() {
            return Err(Box::new(MyError::new("Option out of bounds")));
        }
        let option = &select_menu.options[option];
        if select_menu.component_type == ComponentType::SelectMenu && select_menu.disabled == false
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
                    component_type: select_menu.component_type,
                    custom_id: select_menu.custom_id.as_ref().unwrap().to_string(),
                    type_type: Some(3),
                    values: Some(vec![option.value.to_string()]),
                },
            })?;
            let http = client.http.clone();
            drop(client);

            http.post("https://discord.com/api/v9/interactions")
                .body(body)
                .send()
                .await?;
        }
        Ok(())
    }

    pub async fn reply(&self, content: &str) -> Result<DiscordMessage, Box<dyn std::error::Error>> {
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
            .await?;

        return Ok(self.new_from(serde_json::from_str(&response.text().await?)?));
    }

    pub async fn send(&self, content: &str) -> Result<DiscordMessage, Box<dyn std::error::Error>> {
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
            .await?;
        return Ok(self.new_from(serde_json::from_str(&response.text().await?)?));
    }

    pub async fn edit(&self, content: &str) -> Result<DiscordMessage, Box<dyn std::error::Error>> {
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
            .await?;

        return Ok(self.new_from(serde_json::from_str(&response.text().await?)?));
    }

    pub async fn await_update(&self) -> Result<DiscordMessage, Box<dyn std::error::Error>> {
        let receiver = self
            .client
            .clone()
            .lock()
            .await
            .message_update_receiver
            .clone();
        loop {
            let message = timeout(Duration::from_secs(10), receiver.recv()).await??;
            if message.data.id == self.data.id {
                break Ok(message);
            }
        }
    }
}
