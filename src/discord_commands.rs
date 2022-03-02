use std::{
    error::Error,
    ops::Range,
    time::{Duration, Instant},
};

use futures::future::BoxFuture;
use log::debug;
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
                (message.is_from_master() || message.is_from_me())
                    && message
                        .data
                        .content
                        .starts_with(&format!("<@!{}> ", message.user.id))
            },
            execute: |message| {
                Box::pin(async {
                    let parts = message.data.content.split(" ").collect::<Vec<&str>>();
                    if parts.len() > 1 {
                        let command = &parts[1];
                        let other = if parts.len() > 2 { &parts[2..] } else { &[] };

                        match *command {
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
                            "say" => {
                                message.send(&other.join(" ")).await?;
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
                    let description = message.data.embeds[0]
                        .description
                        .as_ref()
                        .ok_or("No description")?;
                    let number_regex = Regex::new(r"\*\*(\d+)\*\*").unwrap();
                    let number_string = number_regex.captures(description).unwrap();
                    let number: u8 = number_string[1].parse().expect("not a number");
                    message
                        .click_button(0, if number <= 50 { 2 } else { 0 })
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
                    let kraken_line: &str = content.split("\n").collect::<Vec<&str>>()[1];
                    message
                        .click_button(
                            0,
                            if kraken_line.starts_with("              ") {
                                2
                            } else if kraken_line.starts_with("       ") {
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
                                            .await
                                            .ok();

                                        let updated = updated_message_three.await_update().await?;

                                        // end interaction
                                        if updated.get_component(1, 1).is_some() {
                                            updated_message_three.click_button(1, 1).await?;
                                        } else {
                                            updated_message_three.click_button(0, 2).await?;
                                        }
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
        Command {
            last_called: None,
            command: Some(String::from("pls pet")),
            cooldown: Duration::from_secs(60 * 20),
            matcher: |message| {
                let button = message.get_component(0, 0);
                message.is_from_pepe()
                    && message.embed_title_contains(message.user.username.as_str())
                    && button.is_some()
                    && button.unwrap().label.as_ref().unwrap() == "Feed"
            },
            execute: |message| {
                Box::pin(async {
                    for button_index in 0..3 {
                        message.click_button(0, button_index).await?;
                    }

                    message.click_button(1, 4).await?;
                    Ok(())
                })
            },
        },
        // Daily
        Command {
            last_called: None,
            command: Some(String::from("pls daily")),
            cooldown: Duration::from_secs(3600 * 24),
            matcher: |_message| false,
            execute: |_message| Box::pin(async { Ok(()) }),
        },
        // ## Mini Games ##
        Command {
            // Repeat Words Order
            last_called: None,
            command: None,
            cooldown: Duration::from_secs(0),
            matcher: |message| {
                message.is_from_pepe() && message.data.content.contains("Remember words order!")
            },
            execute: |message| {
                Box::pin(async {
                    let words = &message.data.content.split("\n").collect::<Vec<&str>>()[1..]
                        .iter()
                        .map(|word| word.replace("`", ""))
                        .collect::<Vec<String>>();
                    let updated = message.await_update().await?;
                    if updated
                        .data
                        .content
                        .starts_with(format!("<@!{}>", updated.user.id).as_str())
                    {
                        debug!("trying to solve word order");
                        let row = &updated.data.components[0];
                        for word in words {
                            let pos = row
                                .components
                                .iter()
                                .position(|button| button.label.as_ref().unwrap() == word)
                                .unwrap();
                            updated.click_button(0, pos).await?;
                        }
                    }
                    Ok(())
                })
            },
        },
        Command {
            // Emoji Match
            last_called: None,
            command: None,
            cooldown: Duration::from_secs(0),
            matcher: |message| {
                message.is_from_pepe() && message.data.content.contains("Emoji Match")
            },
            execute: |message| {
                Box::pin(async {
                    let (_line, emoji) = message.data.content.split_once("\n").unwrap();
                    let updated = message.await_update().await?;
                    if updated
                        .data
                        .content
                        .starts_with(format!("<@!{}>", updated.user.id).as_str())
                    {
                        debug!("trying to solve Emoji Match");
                        let mut buttons = updated.data.components[0].components.clone();
                        buttons.extend(updated.data.components[1].components.clone());
                        let pos = buttons
                            .iter()
                            .position(|button| button.emoji.as_ref().unwrap().name == emoji)
                            .unwrap();
                        updated
                            .click_button(if pos > 4 { 1 } else { 0 }, pos % 5)
                            .await?;
                    }
                    Ok(())
                })
            },
        },
        Command {
            // Soccer
            last_called: None,
            command: None,
            cooldown: Duration::from_secs(0),
            matcher: |message| message.is_from_pepe() && message.data.content.contains("Soccer"),
            execute: |message| {
                Box::pin(async {
                    // Bot can't know if the message is intended for the user, so it just clicks either way
                    let levitate_line = message.data.content.split("\n").collect::<Vec<&str>>()[2];
                    debug!("trying to solve Soccer");
                    message
                        .click_button(
                            0,
                            if levitate_line.starts_with(":levitate:") {
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
        Command {
            // Color Match
            last_called: None,
            command: None,
            cooldown: Duration::from_secs(0),
            matcher: |message| {
                message.is_from_pepe() && message.data.content.contains("Color Match")
            },
            execute: |message| {
                Box::pin(async {
                    // Bot can't know if the message is intended for the user, so it just clicks either way
                    let lines = message.data.content.split("\n").collect::<Vec<&str>>();
                    let matches = &mut lines[1..4].iter().map(|line| {
                        let (color, word) = line.split_once(" ").unwrap();
                        return ColorMatch {
                            color: color.to_lowercase().chars().nth(2).unwrap(),
                            word: word[1..word.len() - 1].to_owned(),
                        };
                    });

                    debug!(
                        "trying to solve Color Match [1], '{}', '{:#?}'",
                        &message.data.content,
                        matches
                    );

                    let updated = message.await_update().await?;

                    debug!(
                        "trying to solve Color Match [2], '{}'",
                        &updated.data.content
                    );

                    if updated
                        .data
                        .content
                        .starts_with(format!("<@!{}>", updated.user.id).as_str())
                    {
                        debug!("Color Match is for me [3]");

                        let word = &Regex::new(r"`(\w+)`")
                            .unwrap()
                            .captures(&updated.data.content)
                            .unwrap()[1];
                        debug!("Color Match the word is [3], {}", &word);

                        let buttons = &message.data.components[0].components;
                        let color_match = matches
                            .find(|color_match| color_match.word == word)
                            .ok_or("could not find color [1]")?;
                        let index = buttons
                            .iter()
                            .position(|button| {
                                button
                                    .label
                                    .as_ref()
                                    .unwrap()
                                    .to_lowercase()
                                    .starts_with(color_match.color)
                            })
                            .ok_or("could not find color [2]")?;

                        debug!("Color Match index is [4], {}", &index);


                        message.click_button(0, index).await?;
                    }

                    Ok(())
                })
            },
        },
        Command {
            // Dunk the ball
            last_called: None,
            command: None,
            cooldown: Duration::from_secs(0),
            matcher: |message| {
                message.is_from_pepe() && message.data.content.contains("Dunk the ball!")
            },
            execute: |message| {
                Box::pin(async {
                    // Bot can't know if the message is intended for the user, so it just clicks either way
                    let content = &message.data.content;
                    let ball_line: &str = content.split("\n").collect::<Vec<&str>>()[2];

                    debug!("trying to solve Dunk the ball");

                    message
                        .click_button(
                            0,
                            if ball_line.starts_with("              ") {
                                2
                            } else if ball_line.starts_with("       ") {
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
        // Events
        Command {
            // Attack the Boss
            last_called: None,
            command: None,
            cooldown: Duration::from_secs(0),
            matcher: |message| {
                message.is_from_pepe()
                    && message.data.content.starts_with("Attack the boss")
                    && message.get_component(0, 0).is_some()
            },
            execute: |message| {
                Box::pin(async {
                    while {
                        message.click_button(0, 0).await?;

                        let updated = message.await_update().await?;
                        let button = updated.get_component(0, 0);
                        button.is_some() && !button.unwrap().disabled
                    } {}
                    Ok(())
                })
            },
        },
        Command {
            // Trivia Night
            last_called: None,
            command: None,
            cooldown: Duration::from_secs(0),
            matcher: |message| {
                message.is_from_pepe()
                    && message.embed_description_contains("You have 15 seconds to answer")
                    && message.get_component(0, 4).is_some()
            },
            execute: |message| {
                Box::pin(async {
                    message.click_button(0, 0).await?;

                    Ok(())
                })
            },
        },
    ]
}

fn random_range(range: Range<usize>) -> usize {
    (rand::random::<f32>() * range.end as f32).floor() as usize + range.start
}

struct ColorMatch {
    color: char,
    word: String,
}
