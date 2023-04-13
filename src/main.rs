#![feature(once_cell)]
#![allow(warnings)]

use std::collections::HashMap;
use std::env::current_dir;

use serde::Deserialize;
use serenity::prelude::*;
use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;

use crate::handlers::automatic_handler::AutomaticDownloader;

mod downloaders;
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
static CONFIG: &str = include_str!("../resources/properties.toml");
const DISCORD_MAX_FILE_SIZE: u16 = 8;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    //Parsing Toml File
    let config = {
        /*
        let config_file = fs::read("resources/properties.toml").expect(
            "No properties.toml file found in resources folder, please provide a properties file",
        );
        let config_file_utf8 = std::str::from_utf8(config_file.as_ref())
            .expect("Error when parsing properties.toml into Utf8: ");
         */
        toml::from_str::<Config>(CONFIG).expect("Error when parsing toml file:")
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

fn setup_logging() -> WorkerGuard {
    let mut logging_dir = current_dir().unwrap();
    logging_dir.push("log");

    let rolling_file = tracing_appender::rolling::daily(logging_dir, "gamersbot.log");
    let (_non_blocking_stdout, _guard_stdout) = tracing_appender::non_blocking(std::io::stdout());
    let (file_appender, guard) = tracing_appender::non_blocking(rolling_file);
    tracing_subscriber::fmt()
        .with_writer(_non_blocking_stdout)
        .init();
    _guard_stdout
}
