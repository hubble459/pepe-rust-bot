use std::{
    error::Error,
    time::{Duration, Instant},
};

use futures::future::BoxFuture;

use crate::discord_message::DiscordMessage;

#[derive(Clone)]
pub struct Command {
    pub command: String,
    pub cooldown: Duration,
    pub last_called: Option<Instant>,
    pub matcher: fn(&DiscordMessage) -> bool,
    pub execute: for<'a> fn(&'a DiscordMessage) -> BoxFuture<'a, Result<(), Box<dyn Error>>>,
}

pub fn get_commands() -> Vec<Command> {
    vec![
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
            matcher: |message| message.is_pepe_bot() && message.replied_to_me("pls pm") && message.embed_author_contains("meme posting"),
            execute: |message| {
                Box::pin(async {
                    let random = (rand::random::<f32>() * 5f32).floor() as usize;
                    message.click_button(0, random).await;
                    println!("clicked button");
                    let updated = message.wait_update().await;
                    println!("got update");
                    if updated.data.embeds[0]
                        .description
                        .as_ref()
                        .unwrap()
                        .contains("**Laptop** is broken")
                    {
                        updated.send("pls buy laptop").await;
                    }
                    Ok(())
                })
            },
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
    ]
}
