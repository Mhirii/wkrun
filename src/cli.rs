//! Root CLI parser for the wkrun application.
//!
//! The parser is intentionally minimal. Its job is to:
//!
//! - validate root flags (`--help`, `-h`, `--version`, `-V`);
//! - accept bare invocation as a successful parse (the application then
//!   renders the root help itself);
//! - reject unknown arguments and unrecognized subcommands with a
//!   non-zero exit code;
//! - never invoke `std::process::exit`, so observability guards can be
//!   dropped normally during stack unwinding.

use std::ffi::OsString;

/// Root CLI parser.
///
/// The struct currently has no fields: the application exposes only
/// `--help`, `-h`, `--version`, and `-V` at the root. Subcommands will
/// be added here when their behavior is implemented.
#[derive(Debug, clap::Parser)]
#[command(
    name = "wkrun",
    version,
    about = "Local development orchestrator for running and supervising project services.",
    long_about = None,
    disable_help_flag = false,
    disable_version_flag = false,
    arg_required_else_help = false,
)]
pub struct Cli {}

impl Cli {
    /// Parse an explicit argument iterator into a [`Cli`].
    ///
    /// The iterator is consumed exactly as for `Clap::Parser::try_parse_from`,
    /// which means the first element is treated as the program name and
    /// subsequent elements are flags, options, or positional arguments.
    pub fn try_parse_from<I, T>(args: I) -> Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        <Self as clap::Parser>::try_parse_from(args)
    }

    /// Parse the iterator and, on success, also return [`clap::Command`]
    /// used for rendering. Useful for tests that need to inspect or
    /// exercise the underlying command.
    pub fn command() -> clap::Command {
        <Self as clap::CommandFactory>::command()
    }

    /// Render the root help text including the trailing newline that the
    /// plan requires.
    pub fn render_help() -> String {
        let mut command = <Self as clap::CommandFactory>::command();
        let mut help = command.render_help().to_string();
        if !help.ends_with('\n') {
            help.push('\n');
        }
        help
    }

    /// Write the root help text to the provided writer. Used by the
    /// application composition layer to direct help output to stdout
    /// without going through `process::exit`.
    pub fn write_help<W: std::io::Write>(writer: &mut W) -> std::io::Result<()> {
        let mut command = <Self as clap::CommandFactory>::command();
        command
            .write_help(writer)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        writeln!(writer)
    }

    /// Sanitize a [`clap::Error`] into a printable string suitable for
    /// stderr, preserving Clap's intended exit code.
    pub fn format_error(err: &clap::Error) -> String {
        err.to_string()
    }

    /// Whether the clap error represents a successful help request.
    pub fn is_display_help(err: &clap::Error) -> bool {
        err.kind() == clap::error::ErrorKind::DisplayHelp
            || err.kind() == clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    }

    /// Whether the clap error represents a successful version request.
    pub fn is_display_version(err: &clap::Error) -> bool {
        err.kind() == clap::error::ErrorKind::DisplayVersion
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    #[test]
    fn parser_configuration_passes_debug_assert() {
        Cli::command().debug_assert();
    }

    #[test]
    fn long_help_returns_display_help_error() {
        let err =
            Cli::try_parse_from(["wkrun", "--help"]).expect_err("--help must produce clap error");
        assert!(Cli::is_display_help(&err));
        assert_eq!(err.exit_code(), 0);
    }

    #[test]
    fn short_help_returns_display_help_error() {
        let err = Cli::try_parse_from(["wkrun", "-h"]).expect_err("-h must produce clap error");
        assert!(Cli::is_display_help(&err));
        assert_eq!(err.exit_code(), 0);
    }

    #[test]
    fn long_version_returns_display_version_error() {
        let err = Cli::try_parse_from(["wkrun", "--version"])
            .expect_err("--version must produce clap error");
        assert!(Cli::is_display_version(&err));
        assert_eq!(err.exit_code(), 0);
    }

    #[test]
    fn short_version_returns_display_version_error() {
        let err = Cli::try_parse_from(["wkrun", "-V"]).expect_err("-V must produce clap error");
        assert!(Cli::is_display_version(&err));
        assert_eq!(err.exit_code(), 0);
    }

    #[test]
    fn version_string_uses_cargo_pkg_version() {
        let err = Cli::try_parse_from(["wkrun", "--version"])
            .expect_err("--version must produce clap error");
        // Clap renders DisplayVersion using the value configured in the
        // `version` attribute, which reads CARGO_PKG_VERSION when given
        // `version` without an explicit literal.
        let rendered = err.to_string();
        assert!(
            rendered.contains(env!("CARGO_PKG_VERSION")),
            "expected clap version output to include CARGO_PKG_VERSION ({}), got {:?}",
            env!("CARGO_PKG_VERSION"),
            rendered
        );
    }

    #[test]
    fn bare_invocation_parses_successfully() {
        let parsed = Cli::try_parse_from(["wkrun"]).expect("bare invocation must parse");
        // No fields to inspect; the parse is the assertion.
        let _ = parsed;
    }

    #[test]
    fn unknown_option_is_non_zero_and_stderr_directed() {
        let err = Cli::try_parse_from(["wkrun", "--unknown"])
            .expect_err("--unknown must produce clap error");
        assert_eq!(err.kind(), ErrorKind::UnknownArgument);
        assert_ne!(err.exit_code(), 0);
        assert!(err.use_stderr());
    }

    #[test]
    fn unknown_subcommand_is_non_zero_and_stderr_directed() {
        let err = Cli::try_parse_from(["wkrun", "nonexistent"])
            .expect_err("nonexistent must produce clap error");
        assert_ne!(err.exit_code(), 0);
        assert!(err.use_stderr());
    }

    #[test]
    fn rendered_help_contains_required_sections() {
        let help = Cli::render_help();
        assert!(help.contains("Usage:"), "help missing Usage: in {help:?}");
        assert!(help.contains("--help"), "help missing --help in {help:?}");
        assert!(
            help.contains("--version"),
            "help missing --version in {help:?}"
        );
        for placeholder in [
            " up ",
            " down ",
            " logs ",
            " restart ",
            " ls ",
            " attach ",
            " tui ",
        ] {
            assert!(
                !help.contains(placeholder),
                "help unexpectedly contained placeholder command {placeholder:?} in {help:?}"
            );
        }
    }

    #[test]
    fn rendered_help_has_trailing_newline() {
        let help = Cli::render_help();
        assert!(help.ends_with('\n'), "help should end with a newline");
    }
}
