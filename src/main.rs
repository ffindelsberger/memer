use std::format as f;
use std::{env, fs};

use serde::Deserialize;
use serde_json::json;
use serenity::async_trait;
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::model::webhook::Webhook;
use serenity::prelude::*;
use serenity::utils::MessageBuilder;

mod reddit;
mod tiktok;
mod tumblr;
mod youtube;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    // Set a handler for the `message` event - so that whenever a new message
    // is received - the closure (or function) passed will be called.
    //
    // Event handlers are dispatched through a thread-pool, and so multiple
    // events can be dispatched simultaneously.
    async fn message(&self, ctx: Context, msg: Message) {
        //Read context Data first
        let data = ctx.data.read().await;
        let config = data
            .get::<Config>()
            .expect("Expected Config struct in ContextData");

        //println!("{:#?}", msg);

        //If the bot is the author of the user we end here
        if ctx.http.get_current_user().await.unwrap().id == msg.author.id {
            return;
        }

        println!("Got a message");

        //Check for correct channels 1061717451940827326
        if !config.channels_listening.contains(msg.channel_id.as_u64()) {
            println!(
                "The message is not from the meme channel so we dont care about it, lets return"
            );
            return;
        }

        //If something is embedded than we might have to do something if not ignore message and end
        if msg.embeds.is_empty() {
            println!("Nothing is embedded but maybe it did not load fast enough so lets make some extra checks");

            if !(msg.content.contains("youtube") || msg.content.contains("reddit")) {
                println!(
                    "{} - Does not seem to be a url i can work with so we end",
                    msg.content
                );
                return;
            }
            println!("ok its a youtube or reddit link, lets investigate further");
        }

        // First of all lets delete the Message.
        // Even if we fail to convert the Message we dont want the embedded post in the channel
        // but we will send a debug message into the bot channel to tell the user he needs to manually download his post
        let _ = msg.delete(&ctx.http).await;

        let downloaded_file_path = {
            let url = match msg.embeds.is_empty() {
                false => msg.embeds.get(0).unwrap().url.as_ref().unwrap(),
                true => &msg.content,
            };

            let load_result = {
                if url.contains("reddit") {
                    reddit::load(url, &msg).await
                } else if url.contains("youtube") {
                    youtube::load(url, &msg)
                } else {
                    return;
                }
            };

            match load_result {
                Ok(path) => path,
                Err(err) => {
                    send_debug_message(&ctx, err.to_string().as_str(), config.debug).await;
                    return;
                }
            }

            /* let loader: Box<dyn Downloader>;
            if url.contains("youtube") {
                loader = Box::new(YoutubeLoader);
            } else if url.contains("reddit") {
                //loader = Box::new(RedditDownloader);

            } else {
                return;
            }

            match loader.load(&msg.content, &msg) {
                Ok(path) => path,
                Err(err) => {
                    send_debug_message(ctx, err.to_string().as_str());
                    return;
                }
            }*/
        };

        //Validate that file can be sent:
        // - No more than 8GB -> Calculate size in mb - We dont care about rounding down, as long as we get 7 we can send it to Discord
        let size_bytes = downloaded_file_path.metadata().unwrap().len();
        let size_mb = (size_bytes / 1024) / 1024;

        if size_mb >= 8 {
            send_debug_message(
                &ctx,
                &f!(
                    "The File is {}MB large, limit is 8MB so i cant post it",
                    size_mb
                ),
                config.debug,
            )
            .await;
            return;
        }

        //Sending the File to Webhook
        {
            let _payload_data = json!({
                "name": msg.author,
                "avatar":  msg.author.avatar_url().unwrap(),
                "content": "youtube",
            });

            let http_webhook = Http::new("");
            let webhook = Webhook::from_url(&http_webhook, &config.webhook)
                .await
                .expect("Replace the webhook with your own");

            webhook
                .execute(&http_webhook, false, |w| {
                    w.username(&msg.author.name)
                        .avatar_url(&msg.author.avatar_url().unwrap())
                        .add_file(&downloaded_file_path)
                })
                .await
                .expect("Could not execute webhook.");

            /*let form = multipart::Form::new()
                .text("payload_json", payload_data.to_string())
                .file();

            let client = reqwest::Client::new();
            client.post("https://discord.com/api/webhooks/1061792285127356446/39JxqpLESAO-hkgALA4x9Sc0JUWnbyuUSA58sdLysERWk7RwzZPgy0jK8tVwDO5hTCaP")
                .multipart(form)
                .send()
                .await
                .expect("TODO: panic message");*/
        }

        let _msg = msg.channel_id.send_message(&ctx.http, |m| {
            m.content(msg.author.name.to_string())
                .add_file(&downloaded_file_path)
        });

        //I really dont know why but for some reason it puts a space before the filename so we include it here in the delete command
        fs::remove_file(&downloaded_file_path)
            .expect("Error deleting file after uploading it to discord");
    }

    // Set a handler to be called on the `ready` event. This is called when a
    // shard is booted, and a READY payload is sent by Discord. This payload
    // contains data like the current user's guild Ids, current user data,
    // private channels, and more.
    //
    // In this case, just print what the current user's username is.
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

async fn send_debug_message(ctx: &Context, text: &str, channel_id: u64) {
    let response = MessageBuilder::new().push(text).build();
    let _ = ChannelId(channel_id).say(&ctx.http, &response).await;
}

#[derive(Deserialize)]
struct Config {
    channels_listening: Vec<u64>,
    debug: u64,
    webhook: String,
    discord_token: String,
}

impl TypeMapKey for Config {
    type Value = Config;
}

#[tokio::main]
async fn main() {
    // First step :
    //Read Config file and validate if its correct
    //TODO: anstelle von einfach einem expect einen genauen Error ausgeben warum es sich hier um kein korrektes Toml handelt
    // dafür gibt es sicher einen error type in dem toml crate
    let config_file = fs::read("src/resources/properties.toml").expect(
        "No properties.toml file found in resources folder, please provide a properties file",
    );
    let config = toml::from_slice::<Config>(&config_file).expect("Error when parsing toml file");

    // Configure the client with your Discord bot token in the environment.
    //let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let token = &config.discord_token;
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    /* let config = Config {
        channels_listening: vec![695324901044650019],
        debug: 0,
        webhook: "".to_string(),
    };*/

    println!("test");

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .type_map_insert::<Config>(config)
        .await
        .expect("Err creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
