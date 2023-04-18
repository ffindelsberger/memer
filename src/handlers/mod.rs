use std::path::PathBuf;
use std::sync::OnceLock;

use serde_json::json;
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::id::ChannelId;
use serenity::model::prelude::{User, Webhook};
use serenity::prelude::Context;
use serenity::utils::MessageBuilder;

pub mod automatic_handler;

pub static TEMP_DIR: OnceLock<PathBuf> = OnceLock::new();

pub async fn send_debug_message(ctx: &Context, text: &str, channel_id: u64, user: &User) {
    let response = MessageBuilder::new().push(text).mention(user).build();
    let _ = ChannelId(channel_id).say(&ctx.http, &response).await;
}

async fn send_webhook_message(msg: &Message, webhook_url: &str, file_path: &PathBuf) {
    let _payload_data = json!({
        "name": msg.author,
        "avatar":  msg.author.avatar_url().unwrap(),
        "content": "youtube"
    });

    let http_webhook = Http::new("");
    let webhook = Webhook::from_url(&http_webhook, webhook_url)
        .await
        .expect("Replace the webhook with your own");

    webhook
        .execute(&http_webhook, false, |w| {
            w.username(&msg.author.name)
                .avatar_url(&msg.author.avatar_url().unwrap())
                .add_file(file_path)
        })
        .await
        .expect("Could not execute webhook.");
}
