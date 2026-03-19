//! Config loading via shikumi.
//!
//! Provides a single function that encapsulates the config discovery,
//! loading, env-override, and fallback-to-defaults pattern duplicated
//! across 15+ pleme-io applications.

use std::env;
use std::fs;

/// Load application configuration via shikumi discovery.
///
/// This function:
/// 1. Derives the env var name (`{APP_NAME}_CONFIG`) and prefix (`{APP_NAME}_`)
///    from the app name (uppercased, hyphens replaced with underscores).
/// 2. Uses `shikumi::ConfigDiscovery` to find the config file.
/// 3. Loads the config via `shikumi::ConfigStore`, merging env vars.
/// 4. Falls back to `T::default()` if no config file is found.
/// 5. Falls back to a temp file with `{}` if the config file fails to parse.
///
/// # Type Parameters
///
/// - `T`: The config struct. Must implement `Default`, `Clone`, and
///   `serde::Deserialize`. Typically also `Send + Sync + 'static` for
///   use with `ConfigStore`.
pub fn load_config<T>(app_name: &str) -> T
where
    T: Default + Clone + serde::de::DeserializeOwned + Send + Sync + 'static,
{
    let normalized = app_name.to_uppercase().replace('-', "_");
    let env_var = format!("{normalized}_CONFIG");
    let env_prefix = format!("{normalized}_");

    match shikumi::ConfigDiscovery::new(app_name)
        .env_override(&env_var)
        .discover()
    {
        Ok(path) => {
            tracing::info!("loading config from {}", path.display());
            let store = shikumi::ConfigStore::<T>::load(&path, &env_prefix).unwrap_or_else(|e| {
                tracing::warn!("failed to load config: {e}, using defaults");
                let tmp = env::temp_dir().join(format!("{app_name}-default.yaml"));
                fs::write(&tmp, "{}").ok();
                shikumi::ConfigStore::load(&tmp, &env_prefix).unwrap()
            });
            T::clone(&store.get())
        }
        Err(_) => {
            tracing::info!("no config file found, using defaults");
            T::default()
        }
    }
}

/// Load application configuration from a specific file path.
///
/// Unlike [`load_config`], this does not perform discovery. It loads
/// directly from the given path, using the app name only for the env
/// prefix. Falls back to `T::default()` on any error.
pub fn load_config_from_path<T>(app_name: &str, path: &std::path::Path) -> T
where
    T: Default + Clone + serde::de::DeserializeOwned + Send + Sync + 'static,
{
    let normalized = app_name.to_uppercase().replace('-', "_");
    let env_prefix = format!("{normalized}_");

    match shikumi::ConfigStore::<T>::load(path, &env_prefix) {
        Ok(store) => {
            tracing::info!("loaded config from {}", path.display());
            T::clone(&store.get())
        }
        Err(e) => {
            tracing::warn!("failed to load config from {}: {e}, using defaults", path.display());
            T::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[derive(Default, Clone, Debug, Deserialize, PartialEq)]
    struct TestConfig {
        #[serde(default)]
        value: String,
        #[serde(default)]
        count: u32,
    }

    #[test]
    fn load_config_returns_default_when_no_file() {
        let config: TestConfig = load_config("nonexistent_app_shidou_test_12345");
        assert_eq!(config.value, "");
        assert_eq!(config.count, 0);
    }

    #[test]
    fn load_config_returns_default_for_hyphenated_app_name() {
        let config: TestConfig = load_config("non-existent-app-shidou-test");
        assert_eq!(config.value, "");
    }

    #[test]
    fn load_config_reads_yaml_file() {
        let dir = TempDir::new().unwrap();
        let config_dir = dir.path().join("shidou-yaml-test");
        fs::create_dir_all(&config_dir).unwrap();
        let config_file = config_dir.join("shidou-yaml-test.yaml");
        fs::write(&config_file, "value: hello\ncount: 42\n").unwrap();

        let var = "SHIDOU_YAML_TEST_CONFIG";
        unsafe { env::set_var(var, config_file.to_str().unwrap()) };

        let config: TestConfig = load_config("shidou-yaml-test");

        unsafe { env::remove_var(var) };

        assert_eq!(config.value, "hello");
        assert_eq!(config.count, 42);
    }

    #[test]
    fn load_config_reads_toml_file() {
        let dir = TempDir::new().unwrap();
        let config_dir = dir.path().join("shidou-toml-test");
        fs::create_dir_all(&config_dir).unwrap();
        let config_file = config_dir.join("shidou-toml-test.toml");
        fs::write(&config_file, "value = \"toml_val\"\ncount = 7\n").unwrap();

        let var = "SHIDOU_TOML_TEST_CONFIG";
        unsafe { env::set_var(var, config_file.to_str().unwrap()) };

        let config: TestConfig = load_config("shidou-toml-test");

        unsafe { env::remove_var(var) };

        assert_eq!(config.value, "toml_val");
        assert_eq!(config.count, 7);
    }

    #[test]
    fn load_config_falls_back_on_invalid_yaml() {
        let dir = TempDir::new().unwrap();
        let config_dir = dir.path().join("shidou-invalid-test");
        fs::create_dir_all(&config_dir).unwrap();
        let config_file = config_dir.join("shidou-invalid-test.yaml");
        fs::write(&config_file, ": [unclosed\n").unwrap();

        let var = "SHIDOU_INVALID_TEST_CONFIG";
        unsafe { env::set_var(var, config_file.to_str().unwrap()) };

        // Should not panic, falls back to defaults via temp file
        let config: TestConfig = load_config("shidou-invalid-test");

        unsafe { env::remove_var(var) };

        // Falls back to defaults
        assert_eq!(config.value, "");
        assert_eq!(config.count, 0);
    }

    #[test]
    fn load_config_empty_yaml_gives_defaults() {
        let dir = TempDir::new().unwrap();
        let config_dir = dir.path().join("shidou-empty-test");
        fs::create_dir_all(&config_dir).unwrap();
        let config_file = config_dir.join("shidou-empty-test.yaml");
        fs::write(&config_file, "").unwrap();

        let var = "SHIDOU_EMPTY_TEST_CONFIG";
        unsafe { env::set_var(var, config_file.to_str().unwrap()) };

        let config: TestConfig = load_config("shidou-empty-test");

        unsafe { env::remove_var(var) };

        assert_eq!(config.value, "");
        assert_eq!(config.count, 0);
    }

    #[test]
    fn load_config_extra_fields_ignored() {
        let dir = TempDir::new().unwrap();
        let config_dir = dir.path().join("shidou-extra-test");
        fs::create_dir_all(&config_dir).unwrap();
        let config_file = config_dir.join("shidou-extra-test.yaml");
        fs::write(&config_file, "value: known\nunknown_field: ignored\ncount: 3\n").unwrap();

        let var = "SHIDOU_EXTRA_TEST_CONFIG";
        unsafe { env::set_var(var, config_file.to_str().unwrap()) };

        let config: TestConfig = load_config("shidou-extra-test");

        unsafe { env::remove_var(var) };

        assert_eq!(config.value, "known");
        assert_eq!(config.count, 3);
    }

    #[test]
    fn load_config_unicode_values() {
        let dir = TempDir::new().unwrap();
        let config_dir = dir.path().join("shidou-unicode-test");
        fs::create_dir_all(&config_dir).unwrap();
        let config_file = config_dir.join("shidou-unicode-test.yaml");
        fs::write(&config_file, "value: \"始動テスト\"\ncount: 1\n").unwrap();

        let var = "SHIDOU_UNICODE_TEST_CONFIG";
        unsafe { env::set_var(var, config_file.to_str().unwrap()) };

        let config: TestConfig = load_config("shidou-unicode-test");

        unsafe { env::remove_var(var) };

        assert_eq!(config.value, "始動テスト");
        assert_eq!(config.count, 1);
    }

    #[test]
    fn load_config_from_path_reads_yaml() {
        let dir = TempDir::new().unwrap();
        let config_file = dir.path().join("direct.yaml");
        fs::write(&config_file, "value: direct\ncount: 99\n").unwrap();

        let config: TestConfig = load_config_from_path("shidou-direct", &config_file);
        assert_eq!(config.value, "direct");
        assert_eq!(config.count, 99);
    }

    #[test]
    fn load_config_from_path_nonexistent_returns_default() {
        let path = PathBuf::from("/nonexistent/shidou-test-config.yaml");
        let config: TestConfig = load_config_from_path("shidou-nopath", &path);
        assert_eq!(config.value, "");
        assert_eq!(config.count, 0);
    }

    #[test]
    fn load_config_from_path_empty_file_returns_defaults() {
        let dir = TempDir::new().unwrap();
        let config_file = dir.path().join("empty-direct.yaml");
        fs::write(&config_file, "").unwrap();

        let config: TestConfig = load_config_from_path("shidou-empty-direct", &config_file);
        assert_eq!(config.value, "");
        assert_eq!(config.count, 0);
    }

    #[test]
    fn load_config_env_var_name_derived_correctly() {
        // Test that hyphenated names produce correct env var names
        // "my-cool-app" -> "MY_COOL_APP_CONFIG"
        // We can't easily test the env var lookup without setting it,
        // but we can verify the function doesn't panic with various names.
        let _: TestConfig = load_config("a");
        let _: TestConfig = load_config("my-app");
        let _: TestConfig = load_config("my-cool-app-name");
        let _: TestConfig = load_config("ALREADY_UPPER");
    }

    #[test]
    fn load_config_nested_struct() {
        #[derive(Default, Clone, Debug, Deserialize)]
        struct Nested {
            #[serde(default)]
            inner: Inner,
        }

        #[derive(Default, Clone, Debug, Deserialize)]
        struct Inner {
            #[serde(default)]
            depth: u32,
        }

        let dir = TempDir::new().unwrap();
        let config_dir = dir.path().join("shidou-nested-test");
        fs::create_dir_all(&config_dir).unwrap();
        let config_file = config_dir.join("shidou-nested-test.yaml");
        fs::write(&config_file, "inner:\n  depth: 5\n").unwrap();

        let var = "SHIDOU_NESTED_TEST_CONFIG";
        unsafe { env::set_var(var, config_file.to_str().unwrap()) };

        let config: Nested = load_config("shidou-nested-test");

        unsafe { env::remove_var(var) };

        assert_eq!(config.inner.depth, 5);
    }
}
