//! Module registry trait

use crate::module::Module;
use std::sync::Arc;

/// Module registry trait
pub trait ModuleRegistry {

    // Register a module
    fn register(&self, module: Arc<dyn Module>);
}
