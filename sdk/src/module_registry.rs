//! Module registry trait

use crate::message_bus::MessageBounds;
use crate::module::Module;
use std::sync::Arc;

/// Module registry trait
pub trait ModuleRegistry<M: MessageBounds> {
    // Register a module
    fn register(&mut self, module: Arc<dyn Module<M>>);
}
