use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::Client;
use serenity::model::channel::Message;

const VIDEO_FILE_NAME: &str = "video.mp4";
const AUDIO_FILE_NAME: &str = "audio.mp4";

pub async fn load(url: &str, msg: &Message) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    let client = Client::new();
    let res = client.get(url)
            .header("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36")
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap();

    println!("{:#?}", extract_video_url_from_reddit_response(&res));

    let title = extract_video_name_from_reddit_response(&res);
    //Get Video and Audio Url
    let vid_url = extract_video_url_from_reddit_response(&res);
    let mut audio_url = vid_url.split_inclusive('/').collect::<Vec<&str>>();
    let position = audio_url.len() - 1;
    audio_url[position] = "DASH_audio.mp4?source=fallback";

    let audio_url = audio_url
        .iter()
        .fold("".to_string(), |current, &next| current + next);

    println!("{:#?}", audio_url);

    //Download both files
    let vid = client.get(vid_url).send().await.unwrap();

    let filename = vid.headers();
    println!("{:#?}", filename);

    let vid = vid.bytes().await.unwrap();

    let audio = client
        .get(audio_url)
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();

    let vide_path = Path::new("video.mp4");
    let audio_path = Path::new("audio.mp4");

    let mut video_file = File::create(vide_path)?;
    let mut audio_file = File::create(audio_path)?;

    video_file.write_all(vid.as_ref()).unwrap();
    audio_file.write_all(audio.as_ref()).unwrap();

    //Combine audio and video track with ffmpeg
    //lets just add a timestamp to have unique filenames
    let output_file_name = {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string();
        format!("{}-{}.mp4", title, timestamp)
    };

    let mut handle = Command::new("ffmpeg")
        .args([
            "-i",
            VIDEO_FILE_NAME,
            "-i",
            AUDIO_FILE_NAME,
            "-c",
            "copy",
            &output_file_name,
        ])
        .spawn()
        .unwrap();
    //let _ = handle.wait();

    //fs::remove_file(vide_path).unwrap();
    //fs::remove_file(audio_path).unwrap();

    let path = Path::new(&output_file_name);
    Ok(PathBuf::from(path))
}

fn extract_video_url_from_reddit_response(json: &serde_json::Value) -> String {
    //TODO: error handling if the post has no video
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
        .get("secure_media")
        .unwrap()
        .as_object()
        .unwrap()
        .get("reddit_video")
        .unwrap()
        .as_object()
        .unwrap()
        .get("fallback_url")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string()
}

fn extract_video_name_from_reddit_response(json: &serde_json::Value) -> String {
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
