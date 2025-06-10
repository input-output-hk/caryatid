//! Definition of a Caryatid module

use crate::context::Context;
use crate::message_bus::MessageBounds;
use anyhow::Result;
use async_trait::async_trait;
use config::Config;
use std::sync::Arc;

#[async_trait]
pub trait Module<M: MessageBounds>: Send + Sync {
    /// Initialise with the given global context, and local config
    async fn init(&self, context: Arc<Context<M>>, config: Arc<Config>) -> Result<()>;

    // Functions implemented by the #[module] macro
    fn get_name(&self) -> &'static str;
    fn get_description(&self) -> &'static str;
}
