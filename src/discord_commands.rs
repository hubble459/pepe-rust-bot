use std::{
    error::Error,
    ops::Range,
    time::{Duration, Instant},
};

use futures::future::BoxFuture;
use regex::Regex;

use crate::{
    discord_message::DiscordMessage,
    model::{MasterCommand, MasterCommandType},
};

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
        // Master Controls
        Command {
            last_called: None,
            command: None,
            cooldown: Duration::default(),
            matcher: |message| {
                message.is_from_master()
                    && message
                        .data
                        .content
                        .starts_with(&format!("<@!{}> ", message.user.id))
            },
            execute: |message| {
                Box::pin(async {
                    let parts = message.data.content.split_once(" ");
                    if parts.is_some() {
                        let (_mention, command) = parts.unwrap();
                        match command {
                            "start" => {
                                message
                                    .client
                                    .clone()
                                    .lock()
                                    .await
                                    .master_command_sender
                                    .send(MasterCommand {
                                        command: MasterCommandType::Start,
                                        tag: Some(message.data.channel_id.to_string()),
                                    })
                                    .await?;
                            }
                            "stop" => {
                                message
                                    .client
                                    .clone()
                                    .lock()
                                    .await
                                    .master_command_sender
                                    .send(MasterCommand {
                                        command: MasterCommandType::Stop,
                                        tag: None,
                                    })
                                    .await?;
                            }
                            _ => {
                                message.reply(":pleading_face:").await?;
                            }
                        }
                    }
                    Ok(())
                })
            },
        },
        // High Low
        Command {
            last_called: None,
            command: Some(String::from("pls hl")),
            cooldown: Duration::from_secs(30),
            matcher: |message| {
                message.is_from_pepe()
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
                message.is_from_pepe()
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
                message.is_from_pepe()
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
        // Trivia
        Command {
            last_called: None,
            command: Some(String::from("pls trivia")),
            cooldown: Duration::from_secs(5),
            matcher: |message| {
                message.is_from_pepe()
                    && message.replied_to_me("pls trivia")
                    && message.embed_author_contains("trivia question")
            },
            execute: |message| {
                Box::pin(async {
                    message.click_button(0, random_range(0..4)).await?;
                    Ok(())
                })
            },
        },
        // Post Memes
        Command {
            last_called: None,
            command: Some(String::from("pls pm")),
            cooldown: Duration::from_secs(30),
            matcher: |message| {
                message.is_from_pepe()
                    && message.replied_to_me("pls pm")
                    && message.embed_author_contains("meme posting")
            },
            execute: |message| {
                Box::pin(async {
                    message.click_button(0, random_range(0..5)).await?;
                    let updated = message.await_update().await?;
                    if updated.embed_description_contains("**Laptop** is broken") {
                        updated.send("pls buy laptop").await?;
                    }
                    Ok(())
                })
            },
        },
        // Stream
        Command {
            last_called: None,
            command: Some(String::from("pls stream")),
            cooldown: Duration::from_secs(60 * 10),
            matcher: |message| {
                message.is_from_pepe()
                    && message.embed_author_contains(&format!(
                        "{}'s Stream Manager",
                        message.user.username
                    ))
            },
            execute: |message| {
                Box::pin(async {
                    let button = message.get_component(0, 0);
                    match button {
                        Some(button) => {
                            match button.label.unwrap().as_str() {
                                "Go Live" => {
                                    // Start Stream
                                    if !button.disabled {
                                        // click start
                                        message.click_button(0, 0).await?;
                                        // await update
                                        let updated_message = message.await_update().await?;
                                        // choose game
                                        let game_row = updated_message.get_component(0, 0).unwrap();
                                        updated_message
                                            .select_option(
                                                0,
                                                random_range(0..game_row.options.len()),
                                            )
                                            .await?;
                                        // await update
                                        let updated_message_two = message.await_update().await?;
                                        // click start
                                        updated_message_two.click_button(1, 0).await?;
                                        // await update
                                        let updated_message_three =
                                            updated_message_two.await_update().await?;
                                        // click one of the stream buttons
                                        updated_message_three
                                            .click_button(0, random_range(0..3))
                                            .await?;
                                        // end interaction
                                        updated_message_three.click_button(1, 1).await?;
                                    } else {
                                        // can't stream
                                        message.click_button(0, 2).await?;

                                    }
                                }
                                "Run AD" => {
                                    // Is Streaming
                                    if !button.disabled {
                                        message.click_button(0, random_range(0..3)).await?;
                                    }
                                    message.click_button(1, 1).await?;
                                }
                                _ => {}
                            }
                        }
                        None => {}
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

fn random_range(range: Range<usize>) -> usize {
    (rand::random::<f32>() * range.end as f32).floor() as usize + range.start
}
