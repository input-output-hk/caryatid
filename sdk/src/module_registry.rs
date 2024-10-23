//! Module registry for automatic module registration

use crate::module::Module;
use crate::context::Context;
use crate::config::get_sub_config;
use config::Config;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::{info, warn};

/// Static list of modules
static MODULE_REGISTRY: Mutex<Vec<Arc<dyn Module>>>
    = Mutex::new(Vec::new());

pub fn register_module(module: Arc<dyn Module>) {
    MODULE_REGISTRY.lock().unwrap().push(module);
}

pub fn initialise_modules(context: Arc<Context>, config: Arc<Config>) {
    info!("Initialising modules");
    for module in MODULE_REGISTRY.lock().unwrap().iter() {
        let name = module.get_name();
        let config = Arc::new(get_sub_config(&config, name));

        // Only init if enabled
        match config.get_bool("enabled") {
            Ok(true) => {
                info!("Initialising {}", name);
                module.init(context.clone(), config.clone()).unwrap();
            },
            _ => warn!("Ignoring disabled module {}", name),
        }
    }
}
