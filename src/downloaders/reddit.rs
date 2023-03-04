use std::env::temp_dir;
use std::fs;
use std::fs::{canonicalize, File};
use std::io::Write;
use std::ops::Add;
use std::path::PathBuf;
use std::process::Command;

use reqwest::Client;
use serenity::model::channel::Message;
use tracing::info;
use uuid::Uuid;

use crate::downloaders::loaderror::LoadResult;
use crate::downloaders::mbyte_to_byte;
use crate::downloaders::reddit::RedditFileUrl::{Image, Video};

enum RedditFileUrl {
    Image(String),
    Video(String),
}

pub async fn load(url: &str, _msg: &Message, max_filesize: u64) -> LoadResult<PathBuf> {
    let client = Client::new();

    let json_url = {
        if url.ends_with('/') {
            format!("{}{}", &url[0..url.len() - 2], ".json")
        } else if url.contains('?') {
            let split = url.split('?').next().unwrap();
            format!("{}{}", split, "/.json")
        } else {
            format!("{}{}", &url[0..url.len() - 2], "/.json")
        }
    };

    let res = client.get(json_url)
        .header("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36")
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    //lets put an Uuid at the beginning of the filename to prevent filename conflicts
    let output_file_name = Uuid::new_v4().to_string();

    let working_dir = canonicalize(temp_dir())?;
    let output_file_name = match extraxt_file_url_from_reddit_response(&res) {
        Ok(Image(url)) => {
            let image = client.get(url).send().await?.bytes().await.unwrap();

            let path = output_file_name + ".webp";
            File::create(working_dir.join(&path))?.write_all(&image)?;

            path
        }
        Ok(Video(vid_url)) => {
            //Get Video and Audio Url
            let mut audio_url = vid_url.split_inclusive('/').collect::<Vec<&str>>();
            let position = audio_url.len() - 1;
            audio_url[position] = "DASH_audio.mp4?source=fallback";

            let audio_url = audio_url
                .iter()
                .fold("".to_string(), |current, &next| current + next);

            //Download both files
            let vid = client.get(vid_url).send().await?;

            //Check the content_length header to determine if we should event download the file or if it will be too large anyway
            let size = vid
                .content_length()
                .ok_or("Failed to read Content_length Header from Reddit Download")?;
            info!("The Content_length header is {}", size);

            if size > mbyte_to_byte(max_filesize) {
                return Err(format!(
                    "Reddit File is over the Limit of {} Megabytes",
                    max_filesize
                )
                .into());
            }

            let vid = vid.bytes().await.unwrap();
            let audio = client.get(audio_url).send().await?.bytes().await?;

            let video_file_name = Uuid::new_v4().to_string();
            let audio_file_name = Uuid::new_v4().to_string();
            let video_path = working_dir.join(&video_file_name);
            let audio_path = working_dir.join(&audio_file_name);

            let mut video_file = File::create(&video_path)?;
            let mut audio_file = File::create(&audio_path)?;

            video_file.write_all(vid.as_ref())?;
            audio_file.write_all(audio.as_ref())?;

            //Combine audio and video track using ffmpeg
            let output_file_name = output_file_name.add(".mp4");
            let mut handle = Command::new("ffmpeg")
                .args([
                    "-i",
                    video_file_name.as_str(),
                    "-i",
                    audio_file_name.as_str(),
                    "-c",
                    "copy",
                    output_file_name.as_str(),
                ])
                .current_dir(&working_dir)
                .spawn()?;

            loop {
                match handle.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) => {}
                    Err(err) => return Err(err.into()),
                }
            }

            fs::remove_file(video_path)?;
            fs::remove_file(audio_path)?;

            output_file_name
        }
        Err(err) => {
            return Err(err);
        }
    };
    let path = PathBuf::from(&output_file_name);

    Ok(working_dir.join(path))
}

//This is super ugly and i am pretty sure there is a way better method of handling this.
//The idea is to check for the video url and when none can be found look for the img url
fn extraxt_file_url_from_reddit_response(json: &serde_json::Value) -> LoadResult<RedditFileUrl> {
    Ok(extract_video_url(json).unwrap_or(extract_img_url(json)?))
}

/// Web scraping with rust is pure pain
fn extract_video_url(json: &serde_json::Value) -> LoadResult<RedditFileUrl> {
    //TODO: error handling if the post has no video
    let res = json
        .as_array()
        .ok_or("Could not load from given url, please provide a valid reddit url")?
        .get(0)
        .unwrap()
        .as_object()
        .unwrap()
        .get("data")
        .unwrap()
        .as_object()
        .unwrap()
        .get("children")
        .unwrap()
        .as_array()
        .unwrap()
        .get(0)
        .unwrap()
        .as_object()
        .unwrap()
        .get("data")
        .unwrap()
        .as_object()
        .unwrap()
        .get("secure_media")
        .unwrap()
        .as_object()
        .ok_or("This Reddit Link is not a link to a video, mission abort")?
        .get("reddit_video")
        .ok_or("This Reddit Link is not a link to a video, mission abort")?
        .as_object()
        .unwrap()
        .get("fallback_url")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();

    Ok(Video(res))
}

fn _extract_video_name_from_reddit_response(json: &serde_json::Value) -> String {
    json.as_array()
        .unwrap()
        .get(0)
        .unwrap()
        .as_object()
        .unwrap()
        .get("data")
        .unwrap()
        .as_object()
        .unwrap()
        .get("children")
        .unwrap()
        .as_array()
        .unwrap()
        .get(0)
        .unwrap()
        .as_object()
        .unwrap()
        .get("data")
        .unwrap()
        .as_object()
        .unwrap()
        .get("title")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string()
}

fn extract_img_url(json: &serde_json::Value) -> LoadResult<RedditFileUrl> {
    let url = json
        .as_array()
        .unwrap()
        .get(0)
        .unwrap()
        .as_object()
        .unwrap()
        .get("data")
        .unwrap()
        .as_object()
        .unwrap()
        .get("children")
        .unwrap()
        .as_array()
        .unwrap()
        .get(0)
        .unwrap()
        .as_object()
        .unwrap()
        .get("data")
        .unwrap()
        .as_object()
        .unwrap()
        .get("url")
        .unwrap()
        .as_str()
        .ok_or("Not an Image")?
        .to_string();

    Ok(Image(url))
}

#[cfg(test)]
mod test {
    fn test_reddit_image() {}

    fn test_reddit_video() {}

    fn test_reddit_text() {}

    fn test_extract_image_url() {}
}
