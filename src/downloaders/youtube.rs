use std::env::temp_dir;
use std::fs::canonicalize;
use std::path::PathBuf;
use std::sync::OnceLock;

use serenity::model::channel::Message;
use tokio::process::Command;
use tracing::info;

use crate::downloaders::loaderror::LoadResult;

const MAX_SIZE: &str = "20M";

#[cfg(target_os = "linux")]
const YT_DL: &str = "vendors/yt-dlp_linux";
#[cfg(target_os = "macos")]
const YT_DL: &str = "vendors/yt-dlp_macos";

static COMMAND: OnceLock<Command> = OnceLock::new();

//We are allocating a dynamic PathBuf on the Heap, we could use lifetimes to use a Path Object on the stack instead
pub async fn load(url: &str, msg: &Message, _max_filesize: u64) -> LoadResult<PathBuf> {
    if url.contains("playlist") {
        info!("{} is a playlist, we dont load it", &msg.content);
        return Err(
            "Your link is a Playlist, to prevent spamming of the Discord channel i wont load it"
                .into(),
        );
    }

    let filename = msg.id.to_string();
    let filename = filename.trim();
    let downloaded_file = download_file(url, filename).await?;

    match downloaded_file.exists() {
        true => Ok(downloaded_file),
        false => {
            info!(
                "File {:#?} does not exist, it was probably to large to download",
                downloaded_file
            );
            Err(format!(
                "Your file is over the Limit of {}, please download it manually",
                MAX_SIZE
            )
            .into())
        }
    }
}

async fn download_file(url: &str, filename: &str) -> LoadResult<PathBuf> {
    // Because i am changing the working dir of the Child it does not find the yt-dlp_macos binary so i made the path ot the programm also canonical
    // There may be a way better method to solve this problem
    let binary_path = canonicalize(PathBuf::from(YT_DL))?;
    let working_dir = canonicalize(temp_dir())?;
    //TODO: Maybe create a Folder in the temp dir ?
    let cmd = format!(r#"-o {} --max-filesize {}"#, filename, MAX_SIZE);
    let args = shell_words::split(cmd.as_str()).unwrap();

    //We set the working Dir to the tmp dir of the OS to not worry about deleting trash files generated by aborted downloads
    let mut child_handle = Command::new(binary_path)
        .args(args)
        .arg(url)
        .current_dir(&working_dir)
        .spawn()?;

    //Ok so we need to use the Tokio Command module here, std::process::Command blocks the entire process
    let downloaded_file_name = match child_handle.wait().await {
        Ok(status) => {
            if !status.success() {
                return Err(
                    "Could not download from given url, please provide a valid youtube url".into(),
                );
            }
            format!("{}.webm", filename)
        }
        Err(err) => return Err(err.into()),
    };

    Ok(working_dir.join(downloaded_file_name))
}

/*#[cfg(test)]
mod test {
    use std::fs;

    use crate::downloaders::youtube::download_file;

    #[test]
    fn test_download_file_full_video() -> Result<(), String> {
        match download_file("https://www.youtube.com/shorts/B1j3yeHRKbY", "test1") {
            Ok(file_path) => {
                assert!(file_path.exists());
                fs::remove_file(file_path).expect("Panicked while deleting File in test");
            }
            Err(err) => return Err(err.to_string()),
        };

        Ok(())
    }

    fn test_download_youtube_shorts() -> Result<(), String> {
        match download_file("https://www.youtube.com/watch?v=TK4N5W22Gts", "test2") {
            Ok(file_path) => {
                assert!(file_path.exists());
                fs::remove_file(file_path).expect("Panicked while deleting File in test");
            }
            Err(err) => return Err(err.to_string()),
        };

        Ok(())
    }

    #[test]
    fn test_download_youtube_share_link() -> Result<(), String> {
        match download_file("https://youtu.be/TK4N5W22Gts", "test3") {
            Ok(file_path) => {
                assert!(file_path.exists());
                fs::remove_file(file_path).expect("Panicked while deleting File in test");
            }
            Err(err) => return Err(err.to_string()),
        };

        Ok(())
    }
}
*/
