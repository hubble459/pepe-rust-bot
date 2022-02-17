mod discord_client;
mod model;

use dotenv::var;
use crate::discord_client::connect;

#[tokio::main]
async fn main() {
    connect(var("TOKEN").expect("Missing TOKEN in \".env\" file")).await;
}
