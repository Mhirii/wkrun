//! Typed bootstrap errors for the wkrun application entry point.
//!
//! The application bootstrap has a small, fixed surface of real failure
//! modes. The variants here cover only those modes. Speculative variants
//! for config, daemon, IPC, runtime, Docker, Compose, or TUI subsystems
//! are intentionally absent; they will be added when those subsystems
//! are introduced.

use std::io;

/// Concrete error type for application bootstrap operations.
///
/// All error messages are lowercase so they remain grammatical when they
/// become the source of a typed error chain.
#[derive(Debug, thiserror::Error)]
pub enum BootstrapError {
    /// Failed to render the root CLI help to a byte buffer.
    #[error("failed to render root help")]
    RenderHelp(#[source] io::Error),

    /// Failed to write already-rendered output (help or version) to a
    /// standard stream.
    #[error("failed to write application output")]
    WriteOutput(#[source] io::Error),

    /// Failed to install the global tracing subscriber because one is
    /// already installed.
    #[error("a global tracing subscriber is already installed")]
    SubscriberAlreadyInstalled,

    /// Failed to build the runtime filter from a directive string.
    #[error("failed to build runtime tracing filter")]
    BuildFilter(#[source] FilterError),

    /// Failed to install the global tracing subscriber.
    #[error("failed to install global tracing subscriber")]
    InstallSubscriber(String),

    /// Failed to construct the OTLP span exporter.
    #[error("failed to construct otlp span exporter")]
    BuildOtlpExporter(#[source] OtlpExporterError),

    /// Failed to construct the OpenTelemetry tracer provider.
    #[error("failed to construct otlp tracer provider")]
    BuildTracerProvider(#[source] OtlpProviderError),

    /// Failed to flush the OpenTelemetry tracer provider.
    #[error("failed to flush otlp tracer provider")]
    FlushTracer(#[source] OtlpProviderError),

    /// Failed to shut down the OpenTelemetry tracer provider.
    #[error("failed to shut down otlp tracer provider")]
    ShutdownTracer(#[source] OtlpProviderError),

    /// Failed to construct the OpenTelemetry batch processor.
    #[error("failed to construct otel batch processor")]
    BuildOtelProcessor(String),
}

/// Wrapper around [`tracing_subscriber::filter::ParseError`] so the
/// application error surface does not leak the subscriber crate's type
/// hierarchy.
#[derive(Debug)]
pub struct FilterError(pub tracing_subscriber::filter::ParseError);

impl std::fmt::Display for FilterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid tracing filter directive: {}", self.0)
    }
}

impl std::error::Error for FilterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl From<tracing_subscriber::filter::ParseError> for FilterError {
    fn from(err: tracing_subscriber::filter::ParseError) -> Self {
        Self(err)
    }
}

/// Wrapper around [`opentelemetry_otlp::ExporterBuildError`] so the
/// application error surface does not leak the exporter crate's type.
#[derive(Debug)]
pub struct OtlpExporterError(pub opentelemetry_otlp::ExporterBuildError);

impl std::fmt::Display for OtlpExporterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The exporter error already sanitizes its output; do not include
        // any caller-side configuration that might contain credentials.
        write!(f, "otlp exporter construction failed")
    }
}

impl std::error::Error for OtlpExporterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl From<opentelemetry_otlp::ExporterBuildError> for OtlpExporterError {
    fn from(err: opentelemetry_otlp::ExporterBuildError) -> Self {
        Self(err)
    }
}

/// Wrapper around [`opentelemetry_sdk::error::OTelSdkError`] so the
/// application error surface does not leak the SDK crate's type hierarchy.
#[derive(Debug)]
pub struct OtlpProviderError(pub opentelemetry_sdk::error::OTelSdkError);

impl std::fmt::Display for OtlpProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "otlp sdk operation failed")
    }
}

impl std::error::Error for OtlpProviderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl From<opentelemetry_sdk::error::OTelSdkError> for OtlpProviderError {
    fn from(err: opentelemetry_sdk::error::OTelSdkError) -> Self {
        Self(err)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn render_help_display_is_lowercase() {
        let err = BootstrapError::RenderHelp(io::Error::other("test"));
        assert_eq!(err.to_string(), "failed to render root help");
    }

    #[test]
    fn render_help_preserves_source() {
        let io = io::Error::other("disk full");
        let err = BootstrapError::RenderHelp(io);
        let mut current: Option<&(dyn std::error::Error + 'static)> =
            std::error::Error::source(&err);
        let mut chain: Vec<String> = Vec::new();
        while let Some(e) = current {
            chain.push(e.to_string());
            current = std::error::Error::source(e);
        }
        assert_eq!(chain, vec!["disk full".to_string()]);
    }

    #[test]
    fn subscriber_already_installed_has_no_source() {
        let err = BootstrapError::SubscriberAlreadyInstalled;
        assert!(std::error::Error::source(&err).is_none());
    }

    #[test]
    fn otlp_provider_error_chains_source() {
        let sdk_err = opentelemetry_sdk::error::OTelSdkError::InternalFailure("boom".into());
        let err = BootstrapError::ShutdownTracer(OtlpProviderError(sdk_err));
        let _: &dyn std::error::Error = &err;
        let mut current: Option<&(dyn std::error::Error + 'static)> =
            std::error::Error::source(&err);
        let mut chain: Vec<String> = Vec::new();
        while let Some(e) = current {
            chain.push(e.to_string());
            current = std::error::Error::source(e);
        }
        // The internal SDK error must remain reachable from the typed
        // source chain — the test does not pin the exact intermediate
        // rendering the SDK chooses.
        assert!(
            chain.iter().any(|m| m.contains("boom")),
            "expected source chain to retain underlying SDK message, got {chain:?}"
        );
        assert!(
            chain
                .first()
                .map(|m| m == "otlp sdk operation failed")
                .unwrap_or(false),
            "expected wrapper sanitized message first, got {chain:?}"
        );
    }

    #[test]
    fn user_facing_messages_do_not_leak_debug_formatting() {
        let sdk_err = opentelemetry_sdk::error::OTelSdkError::InternalFailure("secret".into());
        let err = BootstrapError::FlushTracer(OtlpProviderError(sdk_err));
        // Top-level message must be the sanitized public message.
        assert!(!err.to_string().contains("secret"));
    }
}
