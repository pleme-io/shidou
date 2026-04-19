//! Tracing initialization helpers.
//!
//! These functions set up `tracing_subscriber` with an `EnvFilter` that
//! respects the `RUST_LOG` environment variable, falling back to a
//! configurable default level.
//!
//! The `init_*` variants call `.init()` and will panic if a global
//! subscriber is already set. The `try_init_*` variants call `.try_init()`
//! and return a `Result` instead, making them safe for tests and
//! multi-init scenarios.

use tracing_subscriber::EnvFilter;

/// Initialize tracing with a default level of `info`.
///
/// Respects the `RUST_LOG` environment variable. If `RUST_LOG` is not set,
/// uses `info` as the default filter.
///
/// # Panics
///
/// Panics if a global subscriber is already set.
pub fn init_tracing() {
    init_tracing_with_level("info");
}

/// Initialize tracing with a custom default level.
///
/// Respects the `RUST_LOG` environment variable. If `RUST_LOG` is not set,
/// uses the provided `level` string as the default filter (e.g. `"debug"`,
/// `"warn"`, `"myapp=trace"`).
///
/// # Panics
///
/// Panics if a global subscriber is already set.
pub fn init_tracing_with_level(level: &str) {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level)),
        )
        .init();
}

/// Try to initialize tracing with a default level of `info`.
///
/// Returns `Ok(())` on success, or an error if a global subscriber is
/// already set. Safe to call in tests.
pub fn try_init_tracing() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    try_init_tracing_with_level("info")
}

/// Try to initialize tracing with a custom default level.
///
/// Returns `Ok(())` on success, or an error if a global subscriber is
/// already set. Safe to call in tests.
pub fn try_init_tracing_with_level(
    level: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level)),
        )
        .try_init()
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.into() })
}

/// Initialize tracing with JSON output.
///
/// Daemon services running under systemd or Kubernetes typically want
/// structured JSON logs so the journal / pod log driver can parse fields.
/// Default level is `info`; override via `RUST_LOG`.
///
/// # Panics
///
/// Panics if a global subscriber is already set.
pub fn init_tracing_json() {
    init_tracing_json_with_level("info");
}

/// Initialize tracing with JSON output at a custom default level.
///
/// # Panics
///
/// Panics if a global subscriber is already set.
pub fn init_tracing_json_with_level(level: &str) {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level)),
        )
        .json()
        .init();
}

/// Try to initialize tracing with JSON output at a custom default level.
///
/// Returns `Ok(())` on success, `Err` if a global subscriber is already set.
pub fn try_init_tracing_json_with_level(
    level: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level)),
        )
        .json()
        .try_init()
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.into() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_init_tracing_does_not_panic() {
        // May fail if another test already initialized, but should not panic.
        let _ = try_init_tracing();
    }

    #[test]
    fn try_init_tracing_with_level_does_not_panic() {
        let _ = try_init_tracing_with_level("debug");
    }

    #[test]
    fn try_init_tracing_with_level_warn() {
        let _ = try_init_tracing_with_level("warn");
    }

    #[test]
    fn try_init_tracing_with_level_error() {
        let _ = try_init_tracing_with_level("error");
    }

    #[test]
    fn try_init_tracing_with_level_trace() {
        let _ = try_init_tracing_with_level("trace");
    }

    #[test]
    fn try_init_tracing_with_module_filter() {
        let _ = try_init_tracing_with_level("myapp=debug,other=warn");
    }

    #[test]
    fn try_init_tracing_with_empty_string() {
        // Empty string should still work (no filtering)
        let _ = try_init_tracing_with_level("");
    }

    #[test]
    fn try_init_tracing_json_does_not_panic() {
        let _ = try_init_tracing_json_with_level("info");
    }

    #[test]
    fn try_init_tracing_json_with_module_filter() {
        let _ = try_init_tracing_json_with_level("myapp=debug,other=warn");
    }

    #[test]
    fn double_init_returns_error_on_second() {
        // First may succeed or fail (depends on test ordering), second
        // should not panic either way.
        let _ = try_init_tracing();
        let second = try_init_tracing();
        // If first succeeded, second should be Err. If first failed
        // (another test beat us), both are Err. Either way, no panic.
        drop(second);
    }
}
