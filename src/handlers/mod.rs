use std::sync::OnceLock;

use serenity::model::id::ChannelId;
use serenity::model::prelude::User;
use serenity::prelude::Context;
use serenity::utils::MessageBuilder;

pub mod automatic_handler;

pub static MAX_FILE_SIZE: OnceLock<u64> = OnceLock::new();

pub async fn send_debug_message(ctx: &Context, text: &str, channel_id: u64, user: &User) {
    let response = MessageBuilder::new().push(text).mention(user).build();
    let _ = ChannelId(channel_id).say(&ctx.http, &response).await;
}
