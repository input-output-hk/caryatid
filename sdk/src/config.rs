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

/// Build a module config by merging the `[global]` section with module-specific config.
///
/// This allows modules to access shared configuration values defined in `[global.*]`
/// while still having their own module-specific overrides. Module-specific values
/// take precedence over global values when keys collide.
///
/// # Example TOML structure
///
/// ```toml
/// [global.startup]
/// method = "default"
/// topic = "app.startup"
///
/// [module.my-module]
/// some-setting = "value"
/// ```
///
/// The resulting module config will contain both `startup.method` and `some-setting`.
pub fn build_module_config(
    root_config: &Config,
    module_table: HashMap<String, config::Value>,
) -> Config {
    Config::builder()
        .add_source(get_sub_config(root_config, "global"))
        .add_source(config_from_value(module_table))
        .build()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::FileFormat;

    fn config_from_toml(toml: &str) -> Config {
        Config::builder()
            .add_source(config::File::from_str(toml, FileFormat::Toml))
            .build()
            .unwrap()
    }

    #[test]
    fn test_get_sub_config_extracts_nested_section() {
        let config = config_from_toml(
            r#"
            [global.startup]
            method = "custom"

            [global.network]
            name = "production"
            "#,
        );

        let global = get_sub_config(&config, "global");

        assert_eq!(global.get_string("startup.method").unwrap(), "custom");
        assert_eq!(global.get_string("network.name").unwrap(), "production");
    }

    #[test]
    fn test_get_sub_config_returns_empty_for_missing_path() {
        let config = config_from_toml("[other]\nkey = \"value\"");
        let sub = get_sub_config(&config, "nonexistent");
        assert!(sub.get_string("anything").is_err());
    }

    #[test]
    fn test_config_from_value_creates_config() {
        let mut map = HashMap::new();
        map.insert("key1".to_string(), config::Value::new(None, "value1"));
        map.insert("key2".to_string(), config::Value::new(None, 42_i64));

        let config = config_from_value(map);

        assert_eq!(config.get_string("key1").unwrap(), "value1");
        assert_eq!(config.get_int("key2").unwrap(), 42);
    }

    #[test]
    fn test_build_module_config_merges_global_and_module() {
        let root_config = config_from_toml(
            r#"
            [global.startup]
            method = "default"
            "#,
        );

        let mut module_table = HashMap::new();
        module_table.insert("name".to_string(), config::Value::new(None, "my-module"));

        let module_cfg = build_module_config(&root_config, module_table);

        assert_eq!(module_cfg.get_string("startup.method").unwrap(), "default");
        assert_eq!(module_cfg.get_string("name").unwrap(), "my-module");
    }

    #[test]
    fn test_build_module_config_module_overrides_global() {
        let root_config = config_from_toml(
            r#"
            [global]
            name = "global-default"
            "#,
        );

        let mut module_table = HashMap::new();
        module_table.insert(
            "name".to_string(),
            config::Value::new(None, "module-override"),
        );

        let module_cfg = build_module_config(&root_config, module_table);

        assert_eq!(module_cfg.get_string("name").unwrap(), "module-override");
    }
}
