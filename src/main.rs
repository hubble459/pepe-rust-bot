mod custom_error;
mod discord_client;
mod discord_commands;
mod discord_message;
mod model;

use crate::discord_client::connect;

use clap::Parser;

/// Automate Dank Memer
#[derive(Parser, Debug)]
#[clap(author = "meep334 <geraldd459@gmail.com>", version = "0.1", about = "Automate the Dank Memer game by using a discord self bot", long_about = None)]
struct Args {
    #[clap(flatten)]
    verbose: clap_verbosity_flag::Verbosity,

    /// Token of the discord account to use
    #[clap(short, long, env)]
    token: String,

    /// The master of this bot (can control the bot)
    #[clap(short, long, env, required = false)]
    master_id: String,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let args = Args::parse();

    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .init();

    connect(args.token, args.master_id).await;
}
