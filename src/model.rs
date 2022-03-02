use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

fn default_as_false() -> bool {
    false
}

fn default_empty_array<T>() -> Vec<T> {
    vec![]
}

#[derive(Debug)]
pub struct MasterCommand {
    pub command: MasterCommandType,
    pub tag: Option<String>,
}

#[repr(u8)]
#[derive(Debug)]
pub enum MasterCommandType {
    Start,
    Stop,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct HelloData {
    pub heartbeat_interval: u64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ResumeData {
    pub token: String,
    pub session_id: String,
    pub seq: u64,
}

#[derive(Serialize, Debug)]
pub struct DiscordMessagePayload {
    pub content: String,
    pub message_reference: Option<DiscordMessagePayloadReference>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DiscordMessagePayloadReference {
    pub channel_id: String,
    pub guild_id: String,
    pub message_id: String,
}

#[derive(Serialize)]
pub struct DiscordMessageInteraction {
    #[serde(rename = "type")]
    pub discord_message_interaction_type: i64,
    pub guild_id: String,
    pub channel_id: String,
    // message_flags: i64,
    pub message_id: String,
    pub application_id: String,
    pub session_id: String,
    pub data: DiscordMessageInteractionComponent,
}

#[derive(Serialize)]
pub struct DiscordMessageInteractionComponent {
    pub component_type: ComponentType,
    pub custom_id: String,
    #[serde(rename = "type")]
    pub type_type: Option<u8>,
    pub values: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IdentifyData {
    pub token: String,
    pub properties: Properties,
    pub compress: bool,
    pub presence: Presence,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReadyData {
    pub user: ReadyDataUser,
    pub session_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReadyDataUser {
    pub accent_color: Option<serde_json::Value>,
    pub avatar: Option<String>,
    pub banner: Option<serde_json::Value>,
    pub banner_color: Option<serde_json::Value>,
    pub bio: String,
    pub desktop: bool,
    pub discriminator: String,
    pub email: String,
    pub flags: i64,
    pub id: String,
    pub mfa_enabled: bool,
    pub mobile: bool,
    pub nsfw_allowed: Option<serde_json::Value>,
    pub phone: Option<serde_json::Value>,
    pub premium: bool,
    pub purchased_flags: i64,
    pub username: String,
    pub verified: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Presence {
    pub activities: Vec<Activity>,
    pub status: String,
    pub since: i64,
    pub afk: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Activity {
    pub name: String,
    #[serde(rename = "type")]
    pub activity_type: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Properties {
    #[serde(rename = "$os")]
    pub os: String,
    #[serde(rename = "$browser")]
    pub browser: String,
    #[serde(rename = "$device")]
    pub device: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageCreateData {
    #[serde(rename = "type")]
    pub message_create_data_type: i64,
    pub tts: bool,
    pub timestamp: String,
    pub referenced_message: Option<Box<MessageCreateData>>,
    pub pinned: bool,
    pub mentions: Vec<MessageCreateDataAuthor>,
    pub mention_roles: Vec<Option<String>>,
    pub mention_everyone: bool,
    pub id: String,
    pub flags: i64,
    pub embeds: Vec<Embed>,
    pub edited_timestamp: Option<String>,
    pub content: String,
    pub components: Vec<MessageCreateDataComponent>,
    pub channel_id: String,
    pub author: MessageCreateDataAuthor,
    pub attachments: Vec<Attachment>,
    pub member: Option<Member>,
    pub guild_id: Option<String>,
    pub reactions: Option<Vec<serde_json::Value>>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Attachment {
    pub content_type: Option<String>,
    pub filename: String,
    pub description: Option<String>,
    pub id: String,
    pub proxy_url: String,
    pub size: i64,
    pub url: String,
    pub height: Option<u32>,
    pub width: Option<u32>,
    pub ephemeral: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageCreateDataAuthor {
    pub username: String,
    pub public_flags: i64,
    pub id: String,
    pub discriminator: String,
    pub bot: Option<bool>,
    pub avatar: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageCreateDataComponent {
    #[serde(rename = "type")]
    pub component_type: ComponentType,
    pub components: Vec<MessageComponent>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessageComponent {
    #[serde(rename = "type")]
    pub component_type: ComponentType,
    pub placeholder: Option<String>,
    #[serde(default = "default_empty_array")]
    pub options: Vec<ComponentOption>,
    pub min_values: Option<i64>,
    pub max_values: Option<i64>,
    pub label: Option<String>,
    pub custom_id: Option<String>,
    pub style: Option<i64>,
    pub emoji: Option<Emoji>,
    #[serde(default = "default_as_false")]
    pub disabled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Emoji {
    pub name: String,
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComponentOption {
    pub value: String,
    pub label: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Embed {
    #[serde(rename = "type")]
    pub embed_type: Option<String>,
    pub author: Option<EmbedAuthor>,
    pub title: Option<String>,
    pub url: Option<String>,
    pub description: Option<String>,
    pub timestamp: Option<String>,
    pub footer: Option<EmbedFooter>,
    pub color: Option<i32>,
    pub image: Option<EmbedImage>,
    pub thumbnail: Option<EmbedThumbnail>,
    pub video: Option<EmbedVideo>,
    pub provider: Option<EmbedProvider>,
    pub fields: Option<Vec<EmbedField>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EmbedAuthor {
    pub name: String,
    pub url: Option<String>,
    pub icon_url: Option<String>,
    pub proxy_icon_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EmbedImage {
    pub url: String,
    pub proxy_url: Option<String>,
    pub height: Option<i32>,
    pub width: Option<i32>,
}

pub type EmbedThumbnail = EmbedImage;

#[derive(Serialize, Deserialize, Debug)]
pub struct EmbedVideo {
    pub url: Option<String>,
    pub proxy_url: Option<String>,
    pub height: Option<i32>,
    pub width: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EmbedFooter {
    pub text: String,
    pub icon_url: Option<String>,
    pub proxy_icon_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EmbedProvider {
    pub text: Option<String>,
    pub url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    pub inline: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Member {
    pub nick: Option<String>,
    pub avatar: Option<String>,
    pub roles: Vec<String>,
    pub mute: bool,
    pub deaf: bool,
    pub joined_at: String,
    pub hoisted_role: Option<serde_json::Value>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PackageWithData<T>
where
    T: Serialize,
{
    pub op: OpCode,
    pub d: T,
    pub s: Option<u64>,
    pub t: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Package {
    pub op: OpCode,
    #[serde(rename = "s")]
    pub sequence: Option<u64>,
    #[serde(rename = "t")]
    pub tag: Option<String>,
    #[serde(rename = "d")]
    pub data: Option<serde_json::Value>,
}

#[derive(Deserialize_repr, Serialize_repr, Debug, Clone, Copy)]
#[repr(u8)]
pub enum OpCode {
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

#[derive(Deserialize_repr, Serialize_repr, Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum ComponentType {
    ActionRow = 1,
    Button = 2,
    SelectMenu = 3,
    TextInput = 4,
}
