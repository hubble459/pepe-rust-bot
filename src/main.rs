mod discord_client;
mod discord_commands;
mod discord_message;
mod model;

use crate::discord_client::connect;
use dotenv::var;

#[tokio::main]
async fn main() {
    connect(
        var("TOKEN").expect("Missing TOKEN in \".env\" file"),
        var("MASTER_ID").expect("Missing MASTER_ID in \".env\" file"),
        var("CHANNEL_ID").expect("Missing CHANNEL_ID in \".env\" file"),
    ).await;
}
