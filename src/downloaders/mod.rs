use std::env::temp_dir;
use std::fs;
use std::path::PathBuf;

use crate::downloaders::loaderror::LoadResult;
use serenity::model::channel::Message;
use tracing::{error, info};

pub mod loaderror;
pub mod reddit;
pub mod tiktok;
pub mod tumblr;
pub mod youtube;

pub enum UrlKind {
    Reddit(String),
    Youtube(String),
}

impl UrlKind {
    pub async fn load(&self, msg: &Message, max_filesize: u64) -> LoadResult<PathBuf> {
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

pub async fn delete_file(path: &PathBuf) {
    //I really dont know why but for some reason it puts a space before the filename so we include it here in the delete command
    info!("Removing {}", path.display());
    match fs::remove_file(path) {
        Ok(_) => {}
        Err(err) => error!(
            "Error deleting file after uploading it to discord : {} with error : {} ",
            path.to_string_lossy(),
            err
        ),
    }
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
