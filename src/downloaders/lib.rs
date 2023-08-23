#![feature(once_cell_try)]

use std::env::temp_dir;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use crate::loaderror::LoadResult;
use serenity::model::channel::Message;

pub mod loaderror;
pub mod reddit;
pub mod tiktok;
pub mod tumblr;
pub mod youtube;

static TEMP_DIR: OnceLock<PathBuf> = OnceLock::new();
pub const DISCORD_MAX_FILE_SIZE_MB: u16 = 8;

pub enum UrlKind {
    Reddit(String),
    Youtube(String),
}

impl UrlKind {
    pub async fn load(&self, msg: &Message, max_filesize: u16) -> LoadResult<PathBuf> {
        match self {
            UrlKind::Reddit(url) => reddit::load(url, msg, max_filesize).await,
            UrlKind::Youtube(url) => youtube::load(url, msg, max_filesize).await,
        }
    }
}

///Converts the given megabyte value to bytes
fn mbyte_to_byte(mbyte: u64) -> u64 {
    (mbyte * 1000) * 1000
}

fn create_working_dir() -> Result<PathBuf, std::io::Error> {
    let mut working_dir = temp_dir();
    working_dir.push("gamersbot_stuff");

    match &working_dir.exists() {
        true => {}
        false => fs::create_dir(&working_dir)?,
    }
    Ok(working_dir)
}
