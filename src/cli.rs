//! Common CLI subcommands for pleme-io applications.
//!
//! Most pleme-io applications share the same set of subcommands: a main
//! GUI/TUI mode (default, no subcommand), an MCP server mode, and a
//! daemon mode. This module provides the `AppCommand` enum that
//! encapsulates these common subcommands.

use clap::Subcommand;

/// Standard subcommands shared by pleme-io applications.
///
/// Applications use this with `#[command(subcommand)]` in their clap CLI
/// struct. When no subcommand is given, the app runs in its default mode
/// (typically GUI or TUI).
///
/// # Example
///
/// ```rust
/// use clap::{Parser, Subcommand};
/// use shidou::AppCommand;
///
/// #[derive(Parser)]
/// struct Cli {
///     #[command(subcommand)]
///     command: Option<AppCommand>,
/// }
/// ```
#[derive(Debug, Clone, Subcommand)]
pub enum AppCommand {
    /// Run as MCP server (stdio transport) for Claude Code integration.
    Mcp,
    /// Run the background daemon.
    Daemon,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    #[command(name = "test-app")]
    struct TestCli {
        #[command(subcommand)]
        command: Option<AppCommand>,
    }

    #[test]
    fn app_command_mcp_parses() {
        let cli = TestCli::try_parse_from(["test-app", "mcp"]).unwrap();
        assert!(matches!(cli.command, Some(AppCommand::Mcp)));
    }

    #[test]
    fn app_command_daemon_parses() {
        let cli = TestCli::try_parse_from(["test-app", "daemon"]).unwrap();
        assert!(matches!(cli.command, Some(AppCommand::Daemon)));
    }

    #[test]
    fn app_command_none_when_no_subcommand() {
        let cli = TestCli::try_parse_from(["test-app"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn app_command_invalid_subcommand_errors() {
        let result = TestCli::try_parse_from(["test-app", "invalid"]);
        assert!(result.is_err());
    }

    #[test]
    fn app_command_mcp_debug_fmt() {
        let cmd = AppCommand::Mcp;
        let debug_str = format!("{cmd:?}");
        assert_eq!(debug_str, "Mcp");
    }

    #[test]
    fn app_command_daemon_debug_fmt() {
        let cmd = AppCommand::Daemon;
        let debug_str = format!("{cmd:?}");
        assert_eq!(debug_str, "Daemon");
    }

    #[test]
    fn app_command_clone() {
        let cmd = AppCommand::Mcp;
        let cloned = cmd.clone();
        assert!(matches!(cloned, AppCommand::Mcp));
    }

    #[test]
    fn app_command_daemon_clone() {
        let cmd = AppCommand::Daemon;
        let cloned = cmd.clone();
        assert!(matches!(cloned, AppCommand::Daemon));
    }

    #[test]
    fn cli_with_help_flag_errors_gracefully() {
        // --help causes an error exit (not a panic)
        let result = TestCli::try_parse_from(["test-app", "--help"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_subcommand_help() {
        let result = TestCli::try_parse_from(["test-app", "mcp", "--help"]);
        assert!(result.is_err());
    }
}
