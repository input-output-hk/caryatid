//! Module registry trait

use crate::module::Module;
use crate::message_bus::MessageBounds;
use std::sync::Arc;

/// Module registry trait
pub trait ModuleRegistry<M: MessageBounds> {

    // Register a module
    fn register(&mut self, module: Arc<dyn Module<M>>);
}
