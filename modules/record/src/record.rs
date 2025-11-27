//! Caryatid Record module

use anyhow::{Error, Result};
use caryatid_sdk::{module, Context, MessageBounds};
use config::Config;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{error, info};

/// Record module
/// Parameterised by the outer message enum used on the bus
#[module(message_type(M), name = "record", description = "Message recorder")]
pub struct Record<M: MessageBounds + serde::Serialize>;

impl<M: MessageBounds + serde::Serialize> Record<M> {
    async fn init(&self, context: Arc<Context<M>>, config: Arc<Config>) -> Result<()> {
        match config.get_string("topic") {
            Ok(topic) => {
                match config.get_string("path") {
                    Ok(path) => {
                        let path = PathBuf::from(&path);
                        let dir = Path::new(&path);
                        if !dir.is_dir() {
                            error!("Path does not exist or is not directory for Record module");
                            return Err(Error::msg(
                                "Path does not exist or is not directory for Record module",
                            ));
                        }
                        info!("Creating message recorder on '{}'", topic);
                        let num = Arc::new(AtomicU64::new(0));
                        let mut subscription = context.subscribe(&topic).await?;
                        context.run(async move {
                            loop {
                                let Ok((_, message)) = subscription.read().await else {
                                    return;
                                };
                                let filename = path
                                    .join(format!("{}.json", num.fetch_add(1, Ordering::SeqCst)));
                                match File::create(&filename) {
                                    Ok(file) => {
                                        match serde_json::to_writer(file, &*message) {
                                            Err(error) => {
                                                error!(
                                                    "Failed to record message to file {:?}: {}",
                                                    &filename, error
                                                );
                                            }
                                            _ => (),
                                        };
                                    }
                                    Err(error) => {
                                        error!(
                                            "Failed to record message to file {:?}: {}",
                                            &filename, error
                                        );
                                    }
                                }
                            }
                        });
                    }
                    _ => error!("No path given for Record module"),
                };
            }

            _ => error!("No topic given for Record module - no effect"),
        }

        Ok(())
    }
}
