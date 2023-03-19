use std::error::Error;
use std::path::PathBuf;

use serenity::model::channel::Message;

//We are allocating a dynamic PathBuf on the Heap, we could use lifetimes to use a Path Object on the stack instead
pub fn _load(_url: &str, _msg: &Message) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    todo!()
}
