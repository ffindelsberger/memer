use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::prelude::Ready;
use serenity::prelude::{Context, EventHandler};
use tracing::{error, info, trace};

use crate::downloaders::loaderror::LoadError;
use crate::downloaders::{delete_file, UrlKind};
use crate::handlers::{send_debug_message, send_webhook_message};
use crate::{Config, DISCORD_MAX_FILE_SIZE};

pub struct AutomaticDownloader;

#[async_trait]
impl EventHandler for AutomaticDownloader {
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

        //If the bot is the author of the user we end here
        let Ok(user) = ctx.http.get_current_user().await else {
            error!("No user found in context with message {} from {}", msg.id, msg.author);
            return;
        };

        if user.id == msg.author.id {
            return;
        }

        //Before i changed the toml properties to a table(channel_id=webhookurl)
        //i had a u64 for the channel_id in my config struct which was much nicer
        //ow i have to do a heap allocation :( just to use the channel_id a String
        let channel_id = msg.channel_id.to_string();
        if !config.channels_listening.contains_key(channel_id.as_str()) {
            trace!(
                "The message is not from the meme channel so we dont care about it, lets return"
            );
            return;
        }

        info!(
            "We got a message from {} with id : {} - {}",
            msg.author, msg.id, msg.content
        );
        //If something is embedded than we might have to do something if not ignore message and end
        if msg.embeds.is_empty() {
            info!("Nothing is embedded but maybe it did not load fast enough so lets make some extra checks");
            if !(msg.content.contains("youtube")
                || msg.content.contains("youtu.be")
                || msg.content.contains("reddit"))
            {
                println!(
                    "{} - Does not seem to be a url i can work with so we end",
                    msg.content
                );
                return;
            }
            info!("ok its a youtube or reddit link, lets investigate further");
        }

        let url = match msg.embeds.is_empty() {
            false => msg.embeds.get(0).unwrap().url.as_ref().unwrap(),
            true => &msg.content,
        };
        let downloaded_file_path = {
            //TODO: clone should not be here
            let url_kind = {
                if url.contains("reddit") && config.downloaders.reddit {
                    UrlKind::Reddit(url.clone())
                } else if url.contains("youtube") && config.downloaders.youtube {
                    UrlKind::Youtube(url.clone())
                } else {
                    return;
                }
            };

            match url_kind.load(&msg, DISCORD_MAX_FILE_SIZE).await {
                Ok(path) => path,
                Err(LoadError::Ignore(reason)) => {
                    info!("Url {url} rejected. Reason: {reason}");
                    return;
                }
                Err(LoadError::Rejected(message)) => {
                    info!("Url {url} rejected. Reason: {message}");
                    send_debug_message(&ctx, message.as_str(), config.debug, &msg.author).await;
                    return;
                }
                Err(LoadError::Error(e)) => {
                    error!("Trying to load file from url {url} resulted in err: {e}");
                    let message = format!(
                        "Internal System Error: User: {} MessageID: {} Url: {}",
                        msg.author, msg.id, url
                    );
                    send_debug_message(&ctx, message.as_str(), config.debug, &msg.author).await;
                    return;
                }
            }
        };

        // Validate that file can be sent:
        // - No more than 25MB -> Calculate size in mb - We dont care about rounding down,
        // as long as we get 24 we can send it to Discord
        let size_in_mb = match downloaded_file_path.metadata() {
            Ok(metadata) => (metadata.len() / 1024) / 1024,
            Err(_) => {
                error!(
                    "File Metadata result returned err, for {} from: {}",
                    downloaded_file_path.to_string_lossy(),
                    url
                );
                return;
            }
        };

        //TODO: Stupid into Conversion from u16 to u64 that is only needed cause i made the const a u16
        if size_in_mb >= DISCORD_MAX_FILE_SIZE.into() {
            send_debug_message(
                &ctx,
                &format!(
                    "The File is {size_in_mb}MB large, limit is {}MB so i cant post it",
                    DISCORD_MAX_FILE_SIZE
                ),
                config.debug,
                &msg.author,
            )
            .await;
            return;
        }

        //TODO: Could not send Webhook error handling
        //Sending the File to Webhook
        send_webhook_message(
            &msg,
            config.channels_listening.get(channel_id.as_str()).unwrap(),
            &downloaded_file_path,
        )
        .await;

        let _msg = msg.channel_id.send_message(&ctx.http, |m| {
            m.content(msg.author.name.to_string())
                .add_file(&downloaded_file_path)
        });

        let _ = msg.delete(&ctx.http).await;
        delete_file(&downloaded_file_path).await;
    }

    // Set a handler to be called on the `ready` event. This is called when a
    // shard is booted, and a READY payload is sent by Discord. This payload
    // contains data like the current user's guild Ids, current user data,
    // private channels, and more.
    //
    // In this case, just print what the current user's username is.
    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} with automatic_handler is connected!", ready.user.name);
    }
}
