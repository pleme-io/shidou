//! `AppRunner` trait and dispatch function.
//!
//! The `AppRunner` trait standardizes the application lifecycle for
//! pleme-io applications. Implement it to get CLI parsing, MCP dispatch,
//! daemon mode, and config loading for free.

use crate::cli::AppCommand;
use crate::config::load_config;
use crate::runtime::create_runtime;

/// Trait for pleme-io applications with standardized lifecycle.
///
/// Implement this to get CLI parsing, MCP dispatch, daemon mode, and
/// config loading for free via the [`dispatch`] function.
///
/// # Type Parameters
///
/// - `Config`: The application's configuration type. Must implement
///   `Default`, `Clone`, and `serde::Deserialize`.
///
/// # Example
///
/// ```rust,no_run
/// use serde::Deserialize;
/// use shidou::AppRunner;
///
/// #[derive(Default, Clone, Debug, Deserialize)]
/// struct Config {
///     #[serde(default)]
///     port: u16,
/// }
///
/// struct MyApp;
///
/// impl AppRunner for MyApp {
///     type Config = Config;
///
///     fn app_name(&self) -> &str { "myapp" }
///
///     fn run_gui(self, config: Self::Config) -> anyhow::Result<()> {
///         println!("running on port {}", config.port);
///         Ok(())
///     }
/// }
/// ```
pub trait AppRunner: Sized {
    /// The app's configuration type.
    type Config: Default + Clone + serde::de::DeserializeOwned + Send + Sync + 'static;

    /// App name (used for config discovery, tracing, etc.).
    fn app_name(&self) -> &str;

    /// Run the main GUI/TUI application.
    fn run_gui(self, config: Self::Config) -> anyhow::Result<()>;

    /// Run the MCP server. Override for custom MCP behavior.
    ///
    /// The default implementation returns an error indicating MCP is not
    /// implemented. Override this and set [`has_mcp`](Self::has_mcp) to
    /// `true` when your app provides an MCP server.
    fn run_mcp(&self) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("MCP server not implemented"))
    }

    /// Run in daemon mode. Override for custom daemon behavior.
    ///
    /// The default implementation returns an error indicating daemon mode
    /// is not implemented. Override this and set
    /// [`has_daemon`](Self::has_daemon) to `true` when your app provides
    /// a daemon mode.
    fn run_daemon(&self, _config: &Self::Config) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("daemon mode not implemented"))
    }

    /// Whether this app supports MCP.
    fn has_mcp(&self) -> bool {
        false
    }

    /// Whether this app supports daemon mode.
    fn has_daemon(&self) -> bool {
        false
    }
}

/// Dispatch function that handles the common CLI pattern.
///
/// Loads config via shikumi, then dispatches to the appropriate runner
/// method based on the command.
///
/// - `None` -> `run_gui(config)`
/// - `Some(AppCommand::Mcp)` -> `run_mcp()` (inside a tokio runtime)
/// - `Some(AppCommand::Daemon)` -> `run_daemon(&config)`
pub fn dispatch<R: AppRunner>(runner: R, command: Option<AppCommand>) -> anyhow::Result<()> {
    let config = load_config::<R::Config>(runner.app_name());
    match command {
        Some(AppCommand::Mcp) => {
            let rt = create_runtime()?;
            rt.block_on(async { runner.run_mcp() })?;
        }
        Some(AppCommand::Daemon) => runner.run_daemon(&config)?,
        None => runner.run_gui(config)?,
    }
    Ok(())
}

/// Dispatch with a pre-loaded config.
///
/// Same as [`dispatch`] but skips config loading. Useful when the caller
/// has already loaded the config or wants to provide a custom config.
pub fn dispatch_with_config<R: AppRunner>(
    runner: R,
    command: Option<AppCommand>,
    config: R::Config,
) -> anyhow::Result<()> {
    match command {
        Some(AppCommand::Mcp) => {
            let rt = create_runtime()?;
            rt.block_on(async { runner.run_mcp() })?;
        }
        Some(AppCommand::Daemon) => runner.run_daemon(&config)?,
        None => runner.run_gui(config)?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::Arc;

    #[derive(Default, Clone, Debug, Deserialize)]
    struct MockConfig {
        #[serde(default)]
        value: String,
    }

    // ── Basic AppRunner implementation for testing ──────────────

    struct MockRunner {
        name: &'static str,
        gui_called: Arc<AtomicBool>,
        mcp_result: Option<anyhow::Result<()>>,
        daemon_result: Option<anyhow::Result<()>>,
    }

    impl MockRunner {
        fn new(name: &'static str) -> (Self, Arc<AtomicBool>) {
            let gui_called = Arc::new(AtomicBool::new(false));
            (
                Self {
                    name,
                    gui_called: gui_called.clone(),
                    mcp_result: None,
                    daemon_result: None,
                },
                gui_called,
            )
        }

        fn with_mcp(mut self, result: anyhow::Result<()>) -> Self {
            self.mcp_result = Some(result);
            self
        }

        fn with_daemon(mut self, result: anyhow::Result<()>) -> Self {
            self.daemon_result = Some(result);
            self
        }
    }

    impl AppRunner for MockRunner {
        type Config = MockConfig;

        fn app_name(&self) -> &str {
            self.name
        }

        fn run_gui(self, _config: Self::Config) -> anyhow::Result<()> {
            self.gui_called.store(true, Ordering::SeqCst);
            Ok(())
        }

        fn run_mcp(&self) -> anyhow::Result<()> {
            match &self.mcp_result {
                Some(Ok(())) => Ok(()),
                Some(Err(_)) => Err(anyhow::anyhow!("mcp error")),
                None => Err(anyhow::anyhow!("MCP server not implemented")),
            }
        }

        fn run_daemon(&self, _config: &Self::Config) -> anyhow::Result<()> {
            match &self.daemon_result {
                Some(Ok(())) => Ok(()),
                Some(Err(_)) => Err(anyhow::anyhow!("daemon error")),
                None => Err(anyhow::anyhow!("daemon mode not implemented")),
            }
        }

        fn has_mcp(&self) -> bool {
            self.mcp_result.is_some()
        }

        fn has_daemon(&self) -> bool {
            self.daemon_result.is_some()
        }
    }

    // ── dispatch tests ──────────────────────────────────────────

    #[test]
    fn dispatch_with_no_command_calls_run_gui() {
        let (runner, gui_called) = MockRunner::new("shidou-dispatch-gui-test");
        let result = dispatch(runner, None);
        assert!(result.is_ok());
        assert!(gui_called.load(Ordering::SeqCst));
    }

    #[test]
    fn dispatch_with_mcp_command_calls_run_mcp() {
        let (runner, gui_called) = MockRunner::new("shidou-dispatch-mcp-test");
        let runner = runner.with_mcp(Ok(()));
        let result = dispatch(runner, Some(AppCommand::Mcp));
        assert!(result.is_ok());
        assert!(!gui_called.load(Ordering::SeqCst));
    }

    #[test]
    fn dispatch_with_daemon_command_calls_run_daemon() {
        let (runner, gui_called) = MockRunner::new("shidou-dispatch-daemon-test");
        let runner = runner.with_daemon(Ok(()));
        let result = dispatch(runner, Some(AppCommand::Daemon));
        assert!(result.is_ok());
        assert!(!gui_called.load(Ordering::SeqCst));
    }

    #[test]
    fn dispatch_mcp_error_propagates() {
        let (runner, _) = MockRunner::new("shidou-dispatch-mcp-err-test");
        let runner = runner.with_mcp(Err(anyhow::anyhow!("mcp fail")));
        let result = dispatch(runner, Some(AppCommand::Mcp));
        assert!(result.is_err());
    }

    #[test]
    fn dispatch_daemon_error_propagates() {
        let (runner, _) = MockRunner::new("shidou-dispatch-daemon-err-test");
        let runner = runner.with_daemon(Err(anyhow::anyhow!("daemon fail")));
        let result = dispatch(runner, Some(AppCommand::Daemon));
        assert!(result.is_err());
    }

    // ── dispatch_with_config tests ──────────────────────────────

    #[test]
    fn dispatch_with_config_no_command() {
        let (runner, gui_called) = MockRunner::new("shidou-dwc-gui-test");
        let config = MockConfig {
            value: "custom".into(),
        };
        let result = dispatch_with_config(runner, None, config);
        assert!(result.is_ok());
        assert!(gui_called.load(Ordering::SeqCst));
    }

    #[test]
    fn dispatch_with_config_mcp() {
        let (runner, _) = MockRunner::new("shidou-dwc-mcp-test");
        let runner = runner.with_mcp(Ok(()));
        let config = MockConfig::default();
        let result = dispatch_with_config(runner, Some(AppCommand::Mcp), config);
        assert!(result.is_ok());
    }

    #[test]
    fn dispatch_with_config_daemon() {
        let (runner, _) = MockRunner::new("shidou-dwc-daemon-test");
        let runner = runner.with_daemon(Ok(()));
        let config = MockConfig::default();
        let result = dispatch_with_config(runner, Some(AppCommand::Daemon), config);
        assert!(result.is_ok());
    }

    // ── AppRunner trait default tests ───────────────────────────

    #[test]
    fn app_runner_default_mcp_returns_error() {
        struct MinimalRunner;
        impl AppRunner for MinimalRunner {
            type Config = MockConfig;
            fn app_name(&self) -> &str {
                "minimal"
            }
            fn run_gui(self, _config: Self::Config) -> anyhow::Result<()> {
                Ok(())
            }
        }

        let runner = MinimalRunner;
        let result = runner.run_mcp();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("MCP server not implemented"), "got: {err_msg}");
    }

    #[test]
    fn app_runner_default_daemon_returns_error() {
        struct MinimalRunner;
        impl AppRunner for MinimalRunner {
            type Config = MockConfig;
            fn app_name(&self) -> &str {
                "minimal"
            }
            fn run_gui(self, _config: Self::Config) -> anyhow::Result<()> {
                Ok(())
            }
        }

        let runner = MinimalRunner;
        let config = MockConfig::default();
        let result = runner.run_daemon(&config);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("daemon mode not implemented"), "got: {err_msg}");
    }

    #[test]
    fn app_runner_default_has_mcp_is_false() {
        struct MinimalRunner;
        impl AppRunner for MinimalRunner {
            type Config = MockConfig;
            fn app_name(&self) -> &str {
                "minimal"
            }
            fn run_gui(self, _config: Self::Config) -> anyhow::Result<()> {
                Ok(())
            }
        }

        let runner = MinimalRunner;
        assert!(!runner.has_mcp());
    }

    #[test]
    fn app_runner_default_has_daemon_is_false() {
        struct MinimalRunner;
        impl AppRunner for MinimalRunner {
            type Config = MockConfig;
            fn app_name(&self) -> &str {
                "minimal"
            }
            fn run_gui(self, _config: Self::Config) -> anyhow::Result<()> {
                Ok(())
            }
        }

        let runner = MinimalRunner;
        assert!(!runner.has_daemon());
    }

    // ── AppRunner with overrides ────────────────────────────────

    #[test]
    fn app_runner_with_mcp_override() {
        struct McpRunner;
        impl AppRunner for McpRunner {
            type Config = MockConfig;
            fn app_name(&self) -> &str {
                "mcp-app"
            }
            fn run_gui(self, _config: Self::Config) -> anyhow::Result<()> {
                Ok(())
            }
            fn run_mcp(&self) -> anyhow::Result<()> {
                Ok(())
            }
            fn has_mcp(&self) -> bool {
                true
            }
        }

        let runner = McpRunner;
        assert!(runner.has_mcp());
        assert!(runner.run_mcp().is_ok());
    }

    #[test]
    fn app_runner_with_daemon_override() {
        struct DaemonRunner;
        impl AppRunner for DaemonRunner {
            type Config = MockConfig;
            fn app_name(&self) -> &str {
                "daemon-app"
            }
            fn run_gui(self, _config: Self::Config) -> anyhow::Result<()> {
                Ok(())
            }
            fn run_daemon(&self, _config: &Self::Config) -> anyhow::Result<()> {
                Ok(())
            }
            fn has_daemon(&self) -> bool {
                true
            }
        }

        let runner = DaemonRunner;
        assert!(runner.has_daemon());
        let config = MockConfig::default();
        assert!(runner.run_daemon(&config).is_ok());
    }

    #[test]
    fn app_runner_with_both_mcp_and_daemon() {
        struct FullRunner;
        impl AppRunner for FullRunner {
            type Config = MockConfig;
            fn app_name(&self) -> &str {
                "full-app"
            }
            fn run_gui(self, _config: Self::Config) -> anyhow::Result<()> {
                Ok(())
            }
            fn run_mcp(&self) -> anyhow::Result<()> {
                Ok(())
            }
            fn run_daemon(&self, _config: &Self::Config) -> anyhow::Result<()> {
                Ok(())
            }
            fn has_mcp(&self) -> bool {
                true
            }
            fn has_daemon(&self) -> bool {
                true
            }
        }

        let runner = FullRunner;
        assert!(runner.has_mcp());
        assert!(runner.has_daemon());
        assert!(runner.run_mcp().is_ok());
        let config = MockConfig::default();
        assert!(runner.run_daemon(&config).is_ok());
    }

    // ── Config interaction tests ────────────────────────────────

    #[test]
    fn dispatch_passes_config_to_gui() {
        static GUI_VALUE: std::sync::OnceLock<String> = std::sync::OnceLock::new();

        struct ConfigCapture;
        impl AppRunner for ConfigCapture {
            type Config = MockConfig;
            fn app_name(&self) -> &str {
                "shidou-config-capture-test"
            }
            fn run_gui(self, config: Self::Config) -> anyhow::Result<()> {
                let _ = GUI_VALUE.set(config.value);
                Ok(())
            }
        }

        // Config will be default since no file exists
        let result = dispatch(ConfigCapture, None);
        assert!(result.is_ok());
        assert_eq!(GUI_VALUE.get().unwrap(), "");
    }

    #[test]
    fn dispatch_with_config_passes_custom_config_to_gui() {
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        struct CountRunner {
            count: Arc<AtomicU32>,
        }
        impl AppRunner for CountRunner {
            type Config = MockConfig;
            fn app_name(&self) -> &str {
                "counter"
            }
            fn run_gui(self, config: Self::Config) -> anyhow::Result<()> {
                self.count.fetch_add(1, Ordering::SeqCst);
                assert_eq!(config.value, "custom-value");
                Ok(())
            }
        }

        let runner = CountRunner {
            count: call_count_clone,
        };
        let config = MockConfig {
            value: "custom-value".into(),
        };
        let result = dispatch_with_config(runner, None, config);
        assert!(result.is_ok());
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    // ── Edge cases ──────────────────────────────────────────────

    #[test]
    fn app_name_can_be_empty() {
        struct EmptyName;
        impl AppRunner for EmptyName {
            type Config = MockConfig;
            fn app_name(&self) -> &str {
                ""
            }
            fn run_gui(self, _config: Self::Config) -> anyhow::Result<()> {
                Ok(())
            }
        }

        let runner = EmptyName;
        assert_eq!(runner.app_name(), "");
        // dispatch should still work (config loading handles empty name)
        let result = dispatch(runner, None);
        assert!(result.is_ok());
    }

    #[test]
    fn gui_error_propagates() {
        struct FailGui;
        impl AppRunner for FailGui {
            type Config = MockConfig;
            fn app_name(&self) -> &str {
                "shidou-fail-gui-test"
            }
            fn run_gui(self, _config: Self::Config) -> anyhow::Result<()> {
                Err(anyhow::anyhow!("gui failed"))
            }
        }

        let result = dispatch(FailGui, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("gui failed"));
    }

    #[test]
    fn default_mcp_dispatch_returns_error() {
        struct NoMcpRunner;
        impl AppRunner for NoMcpRunner {
            type Config = MockConfig;
            fn app_name(&self) -> &str {
                "shidou-no-mcp-test"
            }
            fn run_gui(self, _config: Self::Config) -> anyhow::Result<()> {
                Ok(())
            }
        }

        let result = dispatch(NoMcpRunner, Some(AppCommand::Mcp));
        assert!(result.is_err());
    }

    #[test]
    fn default_daemon_dispatch_returns_error() {
        struct NoDaemonRunner;
        impl AppRunner for NoDaemonRunner {
            type Config = MockConfig;
            fn app_name(&self) -> &str {
                "shidou-no-daemon-test"
            }
            fn run_gui(self, _config: Self::Config) -> anyhow::Result<()> {
                Ok(())
            }
        }

        let result = dispatch(NoDaemonRunner, Some(AppCommand::Daemon));
        assert!(result.is_err());
    }
}
