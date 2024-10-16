// Definition of a Caryatid module

use crate::context::Context;
use anyhow::Result;

pub trait Module: Send + Sync {
    fn init(&self, context: &Context) -> Result<()>;
    fn get_name(&self) -> &'static str;
}

