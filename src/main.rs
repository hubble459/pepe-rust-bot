mod custom_error;
mod discord_client;
mod discord_commands;
mod discord_message;
mod model;

use crate::discord_client::connect;

use clap::Parser;
use clap_verbosity_flag::InfoLevel;

/// Automate Dank Memer
#[derive(Parser, Debug)]
#[clap(author = "meep334 <geraldd459@gmail.com>", version = "1.1", about = "Automate the Dank Memer game by using a discord self bot", long_about = None)]
struct Args {
    #[clap(flatten)]
    verbose: clap_verbosity_flag::Verbosity<InfoLevel>,

    /// Token of the discord account to use
    #[clap(short, long, env)]
    token: String,

    /// The master of this bot (can control the bot)
    #[clap(short, long, env)]
    master_id: Option<String>,

    /// The default channel in which the bot runs
    #[clap(short, long, env)]
    channel_id: Option<String>,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let args = Args::parse();

    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .init();

    connect(args.token, args.master_id, args.channel_id).await;
}
