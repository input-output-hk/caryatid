//! Definition of a Caryatid module

use crate::context::Context;
use anyhow::Result;
use config::Config;

pub trait Module: Send + Sync {
    /// Initialise with the given global context, and local config
    fn init(&self, context: &Context, config: &Config) -> Result<()>;

    // Functions implemented by the #[module] macro
    fn get_name(&self) -> &'static str;
    fn get_description(&self) -> &'static str;
}

