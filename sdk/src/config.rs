use config::Config;
use std::collections::HashMap;

/// Extract a sub-config as a new Config object
/// Defaults to an empty Config if the path does not exist.
pub fn get_sub_config(config: &Config, path: &str) -> Config {
    // Try to extract the sub-config as a table
    match config.get_table(path) {
        Ok(sub_table) => config_from_value(sub_table),
        Err(_) => {
            // Return an empty Config if the path doesn't exist
            Config::default()
        }
    }
}

/// Get a new config from a value map
pub fn config_from_value(map: HashMap<String, config::Value>) -> Config {
    // Use ConfigBuilder to create a new Config with the sub-table
    let mut builder = Config::builder();
    for (key, value) in map.into_iter() {
        builder = builder.set_override(key, value).unwrap();
    }
    builder.build().unwrap_or_default()
}
