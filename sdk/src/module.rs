// Definition of a Caryatid module

use std::sync::Arc;
use crate::context::Context;
use anyhow::Result;

pub trait Module: Send + Sync {
    fn init(&self, context: Arc<Context>) -> Result<()>;
}

