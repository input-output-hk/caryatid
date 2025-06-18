//! Caryatid Playback module

use anyhow::{Error, Result};
use caryatid_sdk::{module, Context, MessageBounds, Module};
use config::Config;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{error, info};

/// Playback module
/// Parameterised by the outer message enum used on the bus
#[module(message_type(M), name = "playback", description = "Message playback")]
pub struct Playback<M: MessageBounds + for<'a> serde::Deserialize<'a>>;

impl<M: MessageBounds + for<'a> serde::Deserialize<'a>> Playback<M> {
    async fn init(&self, context: Arc<Context<M>>, config: Arc<Config>) -> Result<()> {
        match config.get_string("topic") {
            Ok(topic) => {
                match config.get_string("path") {
                    Ok(path) => {
                        let path = PathBuf::from(&path);
                        let dir = Path::new(&path);
                        if !dir.is_dir() {
                            error!("Path does not exist or is not directory for Playback module");
                            return Err(Error::msg(
                                "Path does not exist or is not directory for Playback module",
                            ));
                        }
                        info!("Creating message playback on '{}'", topic);
                        context.run(Self::play_files(context.clone(), topic, path));
                    }
                    _ => error!("No path given for Playback module"),
                };
            }

            _ => error!("No topic given for Playback module - no effect"),
        }

        Ok(())
    }

    async fn play_files(context: Arc<Context<M>>, topic: String, path: PathBuf) -> Result<()> {
        let mut num: u64 = 0;
        loop {
            let filename = path.join(format!("{}.json", num));
            match read_to_string(&filename) {
                Ok(file) => {
                    match serde_json::from_str::<Arc<M>>(&file) {
                        Ok(message) => match context.message_bus.publish(&topic, message).await {
                            Err(error) => {
                                error!("Failed to publish message: {}", error);
                            }
                            _ => (),
                        },
                        Err(error) => {
                            error!(
                                "Failed to playback message to file {:?}: {}. Aborting playback",
                                &filename, error
                            );
                            break;
                        }
                    };
                }
                Err(error) => {
                    error!(
                        "Failed to playback message to file {:?}: {}. Aborting playback",
                        &filename, error
                    );
                    break;
                }
            };
            num = num + 1;
        }
        Ok(())
    }
}
