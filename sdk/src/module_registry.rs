//! Module registry for automatic module registration

use crate::module::Module;
use crate::context::Context;
use config::Config;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::info;

/// Static list of modules
static MODULE_REGISTRY: Mutex<Vec<Arc<dyn Module>>>
    = Mutex::new(Vec::new());

pub fn register_module(module: Arc<dyn Module>) {
    MODULE_REGISTRY.lock().unwrap().push(module);
}

pub fn initialise_modules(context: Arc<Context>, config: Arc<Config>) {
    info!("Initialising modules");
    for module in MODULE_REGISTRY.lock().unwrap().iter() {
        info!("Initialising {}", module.get_name());
        // !!! Get sub-config for modules
        module.init(context.clone(), config.clone()).unwrap();
    }
}
