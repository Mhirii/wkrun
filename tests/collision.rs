//! Integration test for global subscriber collision.
//!
//! `tests/cli.rs` cannot exercise the collision path because each
//! process can install a global subscriber at most once. This file
//! runs as its own integration test binary, so it installs a no-op
//! subscriber first and then asserts that `wkrun::observability::install`
//! returns the typed `SubscriberAlreadyInstalled` error.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use wkrun::error::BootstrapError;
use wkrun::observability;

#[test]
fn install_returns_typed_subscriber_already_installed_when_global_subscriber_is_set() {
    // First install: a no-op global subscriber. This succeeds and
    // pins the global default for the remainder of the test process.
    let _ = tracing::subscriber::set_global_default(tracing_subscriber::Registry::default());

    let config = wkrun::observability::ObservabilityConfig {
        filter: wkrun::observability::DEFAULT_FILTER.to_string(),
        format: wkrun::observability::DiagnosticFormat::Human,
        otlp: None,
        shutdown_budget: std::time::Duration::from_secs(1),
    };

    let result = observability::install(config);
    match result {
        Err(BootstrapError::SubscriberAlreadyInstalled) => {}
        Err(err) => panic!("expected SubscriberAlreadyInstalled, got {err}"),
        Ok(_) => panic!("expected SubscriberAlreadyInstalled, got Ok"),
    }
}
