#![feature(once_cell)]
#![allow(warnings)]

use std::env::current_dir;
use std::fs;

use serde::Deserialize;
use serenity::prelude::*;
use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;

use crate::handlers::automatic_handler::AutomaticDownloader;
use crate::handlers::MAX_FILE_SIZE;

mod downloaders;
mod handlers;

#[derive(Deserialize)]
struct Config {
    channels_listening: Vec<u64>,
    debug: u64,
    webhook: String,
    discord_token: String,
    max_filesize: u64,
    downloaders: Downloaders,
}

#[derive(Deserialize, Default)]
struct Downloaders {
    reddit: bool,
    youtube: bool,
    tiktok: bool,
    tumblr: bool,
}

impl TypeMapKey for Config {
    type Value = Config;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    //TODO: anstelle von einfach einem expect einen genauen Error ausgeben warum es sich hier um kein korrektes Toml handelt
    // dafür gibt es sicher einen error type in dem toml crate

    //Parsing Toml File
    let config = {
        let config_file = fs::read("resources/properties.toml").expect(
            "No properties.toml file found in resources folder, please provide a properties file",
        );
        let config_file_utf8 = std::str::from_utf8(config_file.as_ref())
            .expect("Error when parsing properties.toml into Utf8: ");
        toml::from_str::<Config>(config_file_utf8).expect("Error when parsing toml file:")
    };

    let _logging_guard = setup_logging();

    MAX_FILE_SIZE
        .set(config.max_filesize)
        .expect("Paniced on Setting MAX_FILE_SIZE Constant");
    //Setup Client
    let mut client = {
        // Set gateway intents, which decides what events the bot will be notified about
        let intents = GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::DIRECT_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT;

        // Configure the client with your Discord bot token in the environment.
        //let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
        let token = &config.discord_token;
        // Create a new instance of the Client, logging in as a bot. This will
        // automatically prepend your bot token with "Bot ", which is a requirement
        // by Discord for bot users.
        Client::builder(token, intents)
            .event_handler(AutomaticDownloader)
            .type_map_insert::<Config>(config)
            .await
            .expect("Err creating client")
    };

    // Finally, start a single shard, and start listening to events.
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        info!("Client error: {:?}", why);
    }

    Ok(())
}

fn setup_logging() -> WorkerGuard {
    let rolling_file = tracing_appender::rolling::daily(current_dir().unwrap(), "prefix.log");
    let (_non_blocking_stdout, _guard_stdout) = tracing_appender::non_blocking(std::io::stdout());
    let (file_appender, guard) = tracing_appender::non_blocking(rolling_file);
    tracing_subscriber::fmt().with_writer(file_appender).init();
    guard
}
