//! Application bootstrap for the wkrun binary.
//!
//! This crate composes the parsing, observability, command execution,
//! and shutdown responsibilities of the wkrun entry point. The binary
//! in `src/main.rs` is intentionally minimal: it forwards argument
//! vectors and propagates the resulting `ExitCode`.
//!
//! All subsystem errors carry typed information through
//! [`crate::error`]; the application composition layer preserves those
//! typed sources and source chains through final reporting.

use std::ffi::OsString;
use std::io::Write as _;
use std::process::ExitCode;

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
    let guard = match observability::install(config) {
        Ok(guard) => guard,
        Err(err) => {
            return report_install_failure(err);
        }
    };

    // Render help in an inner scope so the command span is finished and
    // its entry guard dropped before we begin the explicit shutdown.
    let render_result: Result<(), BootstrapError> = {
        let span = tracing::info_span!("cli.command", command = "help");
        let _entered = span.enter();
        tracing::debug!(command = "help", "rendering root help");
        match write_root_help() {
            Ok(()) => Ok(()),
            Err(err) => Err(err),
        }
    };

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

/// Render and write the root help to stdout. Returns the typed
/// `BootstrapError::WriteOutput` on failure so the source chain
/// remains intact.
fn write_root_help() -> Result<(), BootstrapError> {
    let mut stdout = std::io::stdout();
    stdout
        .write_all(Cli::render_help().as_bytes())
        .map_err(BootstrapError::WriteOutput)
}

fn report_install_failure(err: BootstrapError) -> ApplicationOutcome {
    // Observability failed before installation completed; emit a direct
    // stderr fallback so the user sees the failure, then surface a
    // failure outcome. The typed error's Display is sanitized — we do
    // not include any caller-side configuration that might contain
    // credentials.
    eprintln!("error: failed to initialize local diagnostics: {err}");
    ApplicationOutcome::Failure
}

fn report_final_degradation(report: &ShutdownReport) {
    if let Some(message) = report.degradation_message() {
        eprintln!("warning: {message}");
    }
}

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

    #[test]
    fn write_root_help_failure_preserves_io_source() {
        // Render help to a sink that always errors. We exercise the
        // typed path that maps write errors to BootstrapError.
        // (write_root_help writes to stdout, which we cannot redirect
        // from inside this test without closing fd 1; instead, the
        // path is covered by tests/cli.rs at the process level.)
        let err = BootstrapError::WriteOutput(std::io::Error::other("disk full"));
        let mut current: Option<&(dyn std::error::Error + 'static)> =
            std::error::Error::source(&err);
        let mut found = false;
        while let Some(e) = current {
            if e.to_string().contains("disk full") {
                found = true;
                break;
            }
            current = std::error::Error::source(e);
        }
        assert!(found, "WriteOutput must retain its io::Error source");
    }
}
