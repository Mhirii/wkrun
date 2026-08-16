//! Process-level tests that execute the compiled `wkrun` binary and
//! assert on exit codes, stdout, and stderr.
//!
//! These tests rely on Cargo's `CARGO_BIN_EXE_wkrun` environment
//! variable, which is set automatically when the binary is built with
//! Cargo. They use `std::process::Command` directly and avoid any
//! additional binary-testing dependency.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::process::Command;

fn wkrun_binary() -> Command {
    // CARGO_BIN_EXE_wkrun is provided by Cargo for integration tests.
    let path = env!("CARGO_BIN_EXE_wkrun");
    Command::new(path)
}

#[test]
fn long_help_writes_help_to_stdout_and_exits_zero() {
    let output = wkrun_binary()
        .arg("--help")
        .output()
        .expect("run wkrun --help");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Usage:"),
        "stdout missing Usage: {stdout:?}"
    );
    assert!(stdout.contains("--help"));
    assert!(stdout.contains("--version"));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.is_empty(), "stderr should be empty, got {stderr:?}");
}

#[test]
fn short_help_writes_help_to_stdout_and_exits_zero() {
    let output = wkrun_binary().arg("-h").output().expect("run wkrun -h");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.is_empty(), "stderr should be empty, got {stderr:?}");
}

#[test]
fn long_version_writes_version_to_stdout_and_exits_zero() {
    let output = wkrun_binary()
        .arg("--version")
        .output()
        .expect("run wkrun --version");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("wkrun"));
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "stdout missing package version {stdout:?}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.is_empty(), "stderr should be empty, got {stderr:?}");
}

#[test]
fn short_version_writes_version_to_stdout_and_exits_zero() {
    let output = wkrun_binary().arg("-V").output().expect("run wkrun -V");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("wkrun"));
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.is_empty(), "stderr should be empty, got {stderr:?}");
}

#[test]
fn bare_invocation_prints_help_and_exits_zero() {
    let output = wkrun_binary().output().expect("run bare wkrun");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("--help"));
    assert!(stdout.contains("--version"));
}

#[test]
fn unknown_option_writes_to_stderr_and_exits_nonzero() {
    let output = wkrun_binary()
        .arg("--unknown")
        .output()
        .expect("run wkrun --unknown");
    assert_ne!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.is_empty(), "stdout should be empty, got {stdout:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--unknown") || stderr.contains("unexpected argument"),
        "stderr missing clap diagnostic, got {stderr:?}"
    );
    assert!(
        !stderr.contains("panic") && !stderr.contains("backtrace"),
        "stderr leaked panic text, got {stderr:?}"
    );
}

#[test]
fn unknown_subcommand_writes_to_stderr_and_exits_nonzero() {
    let output = wkrun_binary()
        .arg("nonexistent")
        .output()
        .expect("run wkrun nonexistent");
    assert_ne!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("nonexistent") || stderr.contains("unexpected argument"),
        "stderr missing clap diagnostic, got {stderr:?}"
    );
}

#[test]
fn bare_invocation_without_otlp_endpoint_completes_with_zero() {
    // No OTLP env vars set; export must not be attempted.
    let output = wkrun_binary()
        .env_remove("OTEL_EXPORTER_OTLP_ENDPOINT")
        .env_remove("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT")
        .output()
        .expect("run bare wkrun without otlp env");
    assert_eq!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("collector") && !stderr.contains("otlp"),
        "no collector warning expected when export not requested, got {stderr:?}"
    );
}

#[test]
fn bare_invocation_with_unreachable_otlp_endpoint_succeeds_within_budget() {
    // Point OTLP at a deterministic unreachable address with the
    // SDK's default short timeout so shutdown remains bounded.
    //
    // We run the command in a worker thread and wait with a bounded
    // timeout because `std::process::Command::timeout` is not yet
    // stable on every Rust version. If the worker does not finish in
    // time we report a failure rather than hanging the test suite.
    use std::sync::mpsc;
    use std::thread;

    let mut child = wkrun_binary()
        .env("OTEL_EXPORTER_OTLP_ENDPOINT", "http://127.0.0.1:1")
        .env("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT", "http://127.0.0.1:1")
        .env("OTEL_EXPORTER_OTLP_TIMEOUT", "500ms")
        .spawn()
        .expect("spawn wkrun with unreachable otlp");
    let _ = &mut child;

    let (tx, rx) = mpsc::channel();
    let waiter = thread::spawn(move || {
        let result = child.wait_with_output();
        let _ = tx.send(result);
    });

    let bounded = std::time::Duration::from_secs(15);
    let output = match rx.recv_timeout(bounded) {
        Ok(result) => {
            let _ = waiter.join();
            result.expect("wkrun with unreachable otlp completed")
        }
        Err(_) => {
            panic!("wkrun did not complete within {bounded:?} with unreachable otlp");
        }
    };
    assert_eq!(
        output.status.code(),
        Some(0),
        "exporter failure must not change a successful command exit"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Warning may be present but must not contain endpoint details.
    assert!(
        !stderr.contains("127.0.0.1:1"),
        "stderr leaked endpoint details, got {stderr:?}"
    );
}
