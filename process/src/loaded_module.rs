//! Dynamically loaded module
use libloading::{Symbol, Library};
use caryatid_sdk::*;
use anyhow::Result;
use std::sync::Arc;
use config::Config;
use tracing::info;

/// A struct to hold both the dynamically loaded module and the library it
/// depends on.
pub struct LoadedModule {
    pub module: Arc<dyn Module>,
    _lib: Arc<Library>,             // Hold the library to keep it in memory
}

impl LoadedModule {
    /// Load and initialize a module
    pub fn load(lib_name: String, context: &Context, config: &Config)
                -> Result<Self> {
        let module_path = context.config.get_string("paths.modules")
            .unwrap_or(".".to_string());
        let module_file = module_path + "/" + &lib_name;
        info!("Loading module from {}", module_file);

        // Load the library and get the create_module symbol
        let module_lib: Arc<Library>;
        let module_creator: Symbol<fn(&Context, &Config) -> Arc<dyn Module>>;
        unsafe {
            module_lib = Arc::new(Library::new(module_file)?);
            module_creator = module_lib.get(b"create_module")?;
        }

        // Create the module
        let module = module_creator(&context, &config);

        Ok(Self {
            module,
            _lib: module_lib, // Store the library to keep it in scope
        })
    }
}


