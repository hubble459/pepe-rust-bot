use std::{
    error::Error,
    time::{Duration, Instant},
};

use futures::future::BoxFuture;
use regex::Regex;

use crate::discord_message::DiscordMessage;

#[derive(Clone)]
pub struct Command {
    // When command is None it will only be used as a responder and not a command
    pub command: Option<String>,
    pub cooldown: Duration,
    pub last_called: Option<Instant>,
    pub matcher: fn(&DiscordMessage) -> bool,
    pub execute: for<'a> fn(&'a DiscordMessage) -> BoxFuture<'a, Result<(), Box<dyn Error>>>,
}

pub fn get_commands() -> Vec<Command> {
    vec![
        // High Low
        Command {
            last_called: None,
            command: Some(String::from("pls hl")),
            cooldown: Duration::from_secs(30),
            matcher: |message| {
                message.is_pepe_bot()
                    && message.replied_to_me("pls hl")
                    && message.embed_author_contains("high-low")
            },
            execute: |message| {
                Box::pin(async {
                    let description = message.data.embeds[0].description.as_ref().unwrap();
                    let number_regex = Regex::new(r"\*\*(\d+)\*\*").unwrap();
                    let number_string = number_regex.captures(description).unwrap();
                    let number: u8 = number_string[1].parse().expect("not a number");
                    message
                        .click_button(0, if number < 50 { 2 } else { 0 })
                        .await?;
                    Ok(())
                })
            },
        },
        // Hunting
        Command {
            last_called: None,
            command: Some(String::from("pls hunt")),
            cooldown: Duration::from_secs(40),
            matcher: |message| {
                message.is_pepe_bot()
                    && message.replied_to_me("pls hunt")
                    && message.data.content.starts_with("Dodge the Fireball")
            },
            execute: |message| {
                Box::pin(async {
                    let content = &message.data.content;
                    let fireball_line: &str = content.split("\n").collect::<Vec<&str>>()[2];
                    message
                        .click_button(
                            0,
                            if fireball_line.starts_with("       ") {
                                0
                            } else {
                                1
                            },
                        )
                        .await?;
                    Ok(())
                })
            },
        },
        // Fishing
        Command {
            last_called: None,
            command: Some(String::from("pls fish")),
            cooldown: Duration::from_secs(40),
            matcher: |message| {
                message.is_pepe_bot()
                    && message.replied_to_me("pls fish")
                    && message.data.content.starts_with("Catch the fish!")
            },
            execute: |message| {
                Box::pin(async {
                    let content = &message.data.content;
                    let fireball_line: &str = content.split("\n").collect::<Vec<&str>>()[1];
                    message
                        .click_button(
                            0,
                            if fireball_line.starts_with("              ") {
                                2
                            } else if fireball_line.starts_with("       ") {
                                1
                            } else {
                                0
                            },
                        )
                        .await?;
                    Ok(())
                })
            },
        },
        // Digging
        Command {
            last_called: None,
            command: Some(String::from("pls dig")),
            cooldown: Duration::from_secs(40),
            matcher: |_message| false,
            execute: |_message| Box::pin(async { Ok(()) }),
        },
        // Begging
        Command {
            last_called: None,
            command: Some(String::from("pls beg")),
            cooldown: Duration::from_secs(45),
            matcher: |_message| false,
            execute: |_message| Box::pin(async { Ok(()) }),
        },
        // Deposit All
        Command {
            last_called: None,
            command: Some(String::from("pls dep all")),
            cooldown: Duration::from_secs(60),
            matcher: |_message| false,
            execute: |_message| Box::pin(async { Ok(()) }),
        },
        // Post Memes
        Command {
            last_called: None,
            command: Some(String::from("pls pm")),
            cooldown: Duration::from_secs(30),
            matcher: |message| {
                message.is_pepe_bot()
                    && message.replied_to_me("pls pm")
                    && message.embed_author_contains("meme posting")
            },
            execute: |message| {
                Box::pin(async {
                    let random = (rand::random::<f32>() * 5f32).floor() as usize;
                    message.click_button(0, random).await?;
                    let updated = message.wait_update().await?;
                    if updated.embed_description_contains("**Laptop** is broken") {
                        updated.send("pls buy laptop").await?;
                    }
                    Ok(())
                })
            },
        },
        // Stream
        // Command {
        //     last_called: None,
        //     command: Some(String::from("pls stream")),
        //     cooldown: Duration::from_secs(60 * 5),
        //     matcher: |message| {
        //         message.is_pepe_bot()
        //             && message.embed_author_contains(&format!(
        //                 "{}'s Stream Manager",
        //                 message.user.username
        //             ))
        //     },
        //     execute: |message| {
        //         Box::pin(async {
        //             let button = message.get_button(0, 0);
        //             match button {
        //                 Some(button) => {
        //                     // Start Stream
        //                     if button.disabled == true {
        //                         // can't stream
        //                     } else {
        //                         // click start
        //                         // await update
        //                         // choose game
        //                         // await update
        //                         // click start
        //                         // then same as in None case
        //                     }
        //                 }
        //                 None => {
        //                     // Streaming
        //                     // random 0..3
        //                     // 0 ad
        //                     // 1 read
        //                     // 2 donations
        //                 }
        //             }
        //             Ok(())
        //         })
        //     },
        // },
        // Work
        // Command {
        //     last_called: None,
        //     command: String::from("pls work"),
        //     cooldown: Duration::from_secs(3600),
        //     matcher: |_message| false,
        //     execute: |_message| Box::pin(async {Ok(())}),
        // },
        // Pet
        // Command {
        //     last_called: None,
        //     command: String::from("pls pet"),
        //     cooldown: Duration::from_secs(3600),
        //     matcher: |_message| false,
        //     execute: |_message| Box::pin(async {Ok(())}),
        // },
        // Daily
        Command {
            last_called: None,
            command: Some(String::from("pls daily")),
            cooldown: Duration::from_secs(3600 * 24),
            matcher: |_message| false,
            execute: |_message| Box::pin(async { Ok(()) }),
        },
    ]
}
