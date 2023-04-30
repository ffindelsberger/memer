#![feature(once_cell)]
#![feature(once_cell_try)]
#![allow(warnings)]

use std::collections::HashMap;
use std::env::current_dir;

use crate::handlers::automatic_handler::AutomaticDownloader;
use serde::Deserialize;
use serenity::futures::SinkExt;
use serenity::prelude::*;
use tokio::fs;
use tracing::info;
use tracing::instrument::WithSubscriber;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{filter, fmt};

mod handlers;

#[derive(Deserialize)]
struct Config {
    channels_listening: HashMap<String, String>,
    debug: u64,
    discord_token: String,
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

static _CONFIG_FILE_LOCATION: &str = "/etc/opt/gamersbot";
static CONFIG: &str = include_str!("../properties.toml");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    //Parsing Toml File
    let config = {
        /*
        let config_file = fs::read("resources/properties.toml").await.expect(
            "No properties.toml file found, please provide a properties file",
        );
         */
        let config_file_utf8 = std::str::from_utf8(CONFIG.as_bytes())
            .expect("Error when parsing properties.toml into Utf8: ");

        toml::from_str::<Config>(config_file_utf8).expect("Error when parsing toml file:")
    };

    //We have to transfer ownership of the logging guard to the main function,
    //otherwise it will be dropped in the sub-function and we wont have a global
    //logger anymore
    let _logging_guard = setup_logging();

    //Setting up Static Values, maybe put in extra function in the future.
    //It would be better if we init the WORKING_DIR in the dedicated functions, the
    // way it is now i cant test the download functions independently because
    // the WORKING_DIR is not initialized
    //{
    //    MAX_FILE_SIZE
    //        .set(config.max_filesize)
    //        .expect("Panicked on Setting MAX_FILE_SIZE Constant");
    //}

    //Setup Client
    let mut client = {
        // Set gateway intents, which decides what events the bot will be notified about
        let intents = GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::DIRECT_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT;

        // Configure the client with your Discord bot token in the environment.
        //let token = env::var("DISCORD_TOKEN").expect("Expected a token in the
        // environment");
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

/// Returns the Logging Guard which has to be held in Scope to not Drop it, dropping the Logging
/// Guard flushes the current events captures by the logger into the File and makes the Logger
/// unavailable for the rest duration of the Program
fn setup_logging() -> WorkerGuard {
    let mut logging_dir =
        current_dir().expect("Failed to aquire the current dir to be set as logging dir");

    logging_dir.push("log");

    let rolling_file_appender = tracing_appender::rolling::daily(logging_dir, "gamersbot.log");
    let (file_appender, guard) = tracing_appender::non_blocking(rolling_file_appender);
    let stdout = std::io::stdout.with_max_level(tracing::Level::INFO);
    tracing_subscriber::fmt()
        .with_writer(stdout.and(file_appender))
        .init();

    guard
}
