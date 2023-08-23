use std::fs;
use std::fs::File;
use std::io::Write;
use std::ops::Add;
use std::path::PathBuf;

use image::io::Reader as ImageReader;
use image::ImageFormat;
use reqwest::Client;
use serenity::model::channel::Message;
use tokio::process::Command;
use tracing::info;
use uuid::Uuid;

use crate::loaderror::{LoadError, LoadResult};
use crate::reddit::RedditFileUrl::{Image, Video};
use crate::{create_working_dir, mbyte_to_byte, TEMP_DIR};

enum RedditFileUrl {
    Image(String),
    Video(String),
}

pub async fn load(url: &str, _msg: &Message, max_filesize: u16) -> LoadResult<PathBuf> {
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

    let res = client
        .get(json_url)
        .header(
            "user-agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like \
             Gecko) Chrome/108.0.0.0 Safari/537.36",
        )
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let working_dir = TEMP_DIR.get_or_try_init(create_working_dir)?;
    let downloaded_file_path = match extract_file_url_from_reddit_response(&res) {
        Ok(Image(image_url)) => {
            let url = image_url.clone();
            let image_file_extension = image_url.split(".").last().unwrap();
            let image_bytes = client.get(url).send().await?.bytes().await.unwrap();

            let filename = Uuid::new_v4().to_string() + "." + image_file_extension;

            let path = working_dir.join(&filename);
            File::create(&path)?.write_all(&image_bytes)?;

            //We simply try to open the file as an Image, if it fails we wrote the Bytes of
            // a text Post to the File
            let Ok(_) = ImageReader::open(&path)?.decode() else {
                return Err(LoadError::Ignore("This is a text post".into()));
            };
            let Ok(format) = ImageFormat::from_path(&path) else {
                return Err(LoadError::Ignore("This is a text post".into()));
            };

            //Because gifs are so fucking huge we convert the gif to an mp4 file
            let filename = match format {
                ImageFormat::Gif => {
                    return convert_gif_to_mp4(path).await;
                }
                _ => filename,
            };

            PathBuf::from(filename)
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

            //Check the content_length header to determine if we should event download the
            //file or if it will be too large anyway
            let size = vid
                .content_length()
                .ok_or("Failed to read Content_length Header from Reddit Download")?;
            info!("The Content_length header is {}", size);

            if size > mbyte_to_byte(max_filesize.into()) {
                return Err(format!(
                    "Reddit File is over the Limit of {} Megabytes",
                    max_filesize
                )
                .into());
            }

            let video = vid.bytes().await.unwrap();
            let audio = client.get(audio_url).send().await?.bytes().await?;

            let video_path = working_dir.join(Uuid::new_v4().to_string());
            let audio_path = working_dir.join(Uuid::new_v4().to_string());
            let mut video_file = File::create(&video_path)?;
            let mut audio_file = File::create(&audio_path)?;
            video_file.write_all(video.as_ref())?;
            audio_file.write_all(audio.as_ref())?;

            //Combine audio and video track using ffmpeg
            let filename = Uuid::new_v4().to_string().add(".mp4");
            let mut handle = Command::new("ffmpeg")
                .args([
                    "-i",
                    &Uuid::new_v4().to_string(),
                    "-i",
                    &Uuid::new_v4().to_string(),
                    "-c",
                    "copy",
                    filename.as_str(),
                ])
                .current_dir(&working_dir)
                .spawn()?;

            match handle.wait().await {
                Ok(_) => {}
                Err(err) => return Err(err.into()),
            }

            fs::remove_file(video_path)?;
            fs::remove_file(audio_path)?;

            PathBuf::from(filename)
        }
        Err(err) => {
            return Err(err);
        }
    };
    Ok(working_dir.join(downloaded_file_path))
}

async fn convert_gif_to_mp4(path: PathBuf) -> LoadResult<PathBuf> {
    let new_filename = path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .split(".")
        .next()
        .unwrap();
    let new_filename = String::from(new_filename) + ".mp4";

    let mut new_path = PathBuf::from(path.parent().unwrap());
    new_path.push(new_filename);

    let mut handle = Command::new("ffmpeg")
        .args(["-i", path.to_str().unwrap(), new_path.to_str().unwrap()])
        .current_dir(TEMP_DIR.get_or_try_init(create_working_dir)?)
        .spawn()?;

    match handle.wait().await {
        Ok(_) => {}
        Err(err) => return Err(err.into()),
    }

    Ok(new_path)
}

//This is super ugly and i am pretty sure there is a way better method of
// handling this. The idea is to check for the video url and when none can be
// found look for the img url
fn extract_file_url_from_reddit_response(json: &serde_json::Value) -> LoadResult<RedditFileUrl> {
    let result = extract_video_url(json).unwrap_or(extract_img_url(json)?);
    Ok(result)
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

    fn test_reddit_gif() {}
}
