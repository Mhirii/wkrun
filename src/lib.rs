//! Application bootstrap for the wkrun binary.
//!
//! This crate composes the parsing, observability, command execution,
//! and shutdown responsibilities of the wkrun entry point. The binary
//! in `src/main.rs` is intentionally minimal: it forwards argument
//! vectors and propagates the resulting `ExitCode`.
//!
//! This module is the *only* place where `anyhow` is used. All
//! subsystem errors carry typed information through [`crate::error`].

use std::ffi::OsString;
use std::io::Write as _;
use std::process::ExitCode;

use anyhow::Context as _;

use crate::cli::Cli;
use crate::error::BootstrapError;
use crate::observability::{ObservabilityConfig, ShutdownReport};

pub mod cli;
pub mod error;
pub mod observability;

/// Outcome of running the wkrun application to completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicationOutcome {
    /// Command completed successfully.
    Success,
    /// Command failed because the user supplied invalid CLI input.
    /// The wrapped value is the exit code returned by Clap.
    UsageError(u8),
    /// Command failed for an internal/bootstrap reason.
    Failure,
}

impl ApplicationOutcome {
    /// Convert into a [`std::process::ExitCode`].
    pub fn into_exit_code(self) -> ExitCode {
        match self {
            ApplicationOutcome::Success => ExitCode::SUCCESS,
            ApplicationOutcome::UsageError(code) => ExitCode::from(code),
            ApplicationOutcome::Failure => ExitCode::from(1u8),
        }
    }
}

/// Run the wkrun application with the supplied argument vector.
///
/// The first element of `args` is treated as the program name, matching
/// the convention used by `std::env::args_os` and Clap's `try_parse_from`.
pub fn run<I, T>(args: I) -> ApplicationOutcome
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let parse_result = Cli::try_parse_from(args);

    let cli = match parse_result {
        Ok(cli) => cli,
        Err(err) => return handle_parse_error(err),
    };

    // The only successful parse at the root is bare `wkrun`. Render the
    // root help ourselves so we control the stream and exit code and can
    // wrap observability around the operation.
    run_root_help(cli)
}

fn handle_parse_error(err: clap::Error) -> ApplicationOutcome {
    use clap::error::ErrorKind;

    if Cli::is_display_help(&err) || Cli::is_display_version(&err) {
        // Clap renders help/version through its own error printing
        // mechanism; we delegate to it so styling and stream selection
        // match Clap's intended behavior, but we never call exit().
        let _ = err.print();
        return ApplicationOutcome::Success;
    }

    if matches!(
        err.kind(),
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
    ) {
        let _ = err.print();
        return ApplicationOutcome::Success;
    }

    // Any other error is a usage error: stderr, non-zero, no panic.
    let _ = err.print();
    let code = err.exit_code();
    let trimmed = if code < 0 {
        2u8
    } else {
        code.clamp(0, u8::MAX as i32) as u8
    };
    ApplicationOutcome::UsageError(trimmed)
}

fn run_root_help(_cli: Cli) -> ApplicationOutcome {
    // Initialize observability before rendering help so any rendering or
    // shutdown failure is reported through the same channel as other
    // bootstrap operations.
    let config = ObservabilityConfig::from_env();
    let install_result: anyhow::Result<_> =
        observability::install(config).context("failed to initialize local diagnostics");
    let guard = match install_result {
        Ok(guard) => guard,
        Err(err) => {
            return report_install_failure(BootstrapError::InstallSubscriber(err.to_string()));
        }
    };

    let span = tracing::info_span!("cli.command", command = "help");
    let _enter = span.enter();
    tracing::debug!(command = "help", "rendering root help");

    let render_result: anyhow::Result<()> = std::io::stdout()
        .write_all(Cli::render_help().as_bytes())
        .context("failed to render root help");

    let report = guard.shutdown();

    if let Err(err) = render_result {
        // The primary failure is the help-writing failure. Report it
        // directly to stderr and return failure; telemetry shutdown
        // degradation must not replace this signal.
        eprintln!("error: {err}");
        report_final_degradation(&report);
        return ApplicationOutcome::Failure;
    }

    report_final_degradation(&report);
    ApplicationOutcome::Success
}

fn report_install_failure(err: BootstrapError) -> ApplicationOutcome {
    // Observability failed before installation completed; emit a direct
    // stderr fallback so the user sees the failure, then surface a
    // failure outcome.
    eprintln!("error: failed to initialize local diagnostics: {err}");
    ApplicationOutcome::Failure
}

fn report_final_degradation(report: &ShutdownReport) {
    if let Some(message) = report.degradation_message() {
        eprintln!("warning: {message}");
    }
}

// Required to keep `std::io::Write` available without an explicit import
// in the function body where it is used through fully-qualified calls.

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn outcome_success_maps_to_zero() {
        assert_eq!(
            ApplicationOutcome::Success.into_exit_code(),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn outcome_failure_maps_to_one() {
        assert_eq!(
            ApplicationOutcome::Failure.into_exit_code(),
            ExitCode::from(1u8)
        );
    }

    #[test]
    fn outcome_usage_error_preserves_code() {
        assert_eq!(
            ApplicationOutcome::UsageError(2).into_exit_code(),
            ExitCode::from(2u8)
        );
    }
}
