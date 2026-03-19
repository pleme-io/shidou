//! Shidou (始動) — app bootstrap for pleme-io applications.
//!
//! Extracts the duplicated initialization patterns found across 38+ pleme-io
//! repositories into a single, well-tested library:
//!
//! - **Tracing initialization** with `EnvFilter` and configurable default levels
//! - **Config loading** via shikumi with discovery, env overrides, and defaults
//! - **Tokio runtime** creation helpers
//! - **`AppRunner` trait** for standardized app lifecycle
//! - **`AppCommand` enum** for CLI subcommand dispatch
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use clap::{Parser, Subcommand};
//! use serde::Deserialize;
//! use shidou::{AppCommand, AppRunner, dispatch, init_tracing};
//!
//! #[derive(Default, Clone, Debug, Deserialize)]
//! struct Config {
//!     #[serde(default)]
//!     refresh_ms: u32,
//! }
//!
//! struct MyApp;
//!
//! impl AppRunner for MyApp {
//!     type Config = Config;
//!
//!     fn app_name(&self) -> &str { "myapp" }
//!
//!     fn run_gui(self, config: Self::Config) -> anyhow::Result<()> {
//!         println!("running with refresh_ms={}", config.refresh_ms);
//!         Ok(())
//!     }
//! }
//!
//! fn main() -> anyhow::Result<()> {
//!     init_tracing();
//!     dispatch(MyApp, None)
//! }
//! ```

pub mod cli;
pub mod config;
pub mod runner;
pub mod runtime;
pub mod tracing_init;

// Re-export key items at top level for ergonomic usage.
pub use cli::AppCommand;
pub use config::load_config;
pub use runner::{AppRunner, dispatch};
pub use runtime::{block_on, create_runtime};
pub use tracing_init::{init_tracing, init_tracing_with_level, try_init_tracing, try_init_tracing_with_level};
