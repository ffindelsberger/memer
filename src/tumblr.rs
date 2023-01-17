use serenity::model::channel::Message;
use std::error::Error;
use std::path::PathBuf;
use std::process::Command;

//We are allocating a dynamic PathBuf on the Heap, we could use lifetimes to use a Path Object on the stack instead
pub fn load(url: &str, msg: &Message) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    //Download the File from youtube using this python program and save the file with the msg id as filename
    let filename = msg.id.to_string();
    let filename = filename.trim();
    let mut process_handle = Command::new("vendors/yt-dlp_macos")
        .arg(format!("-o {}", filename))
        .arg(url)
        .spawn()
        .unwrap();

    //let _ = process_handle.wait();
    //let downloaded_file_name = format!(" {}.webm", filename);

    let downloaded_file_name = loop {
        match process_handle.try_wait() {
            Ok(Some(_)) => break format!(" {}.webm", filename),
            Ok(None) => {}
            Err(err) => return Err(err.to_string().into()),
        }
    };

    Ok(PathBuf::from(downloaded_file_name))
}
