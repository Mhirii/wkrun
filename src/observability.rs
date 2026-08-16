//! Observability bootstrap for the wkrun application entry point.
//!
//! Responsibilities:
//!
//! - construct the runtime tracing filter from a directive string;
//! - build the human-readable and JSON diagnostic formatting layers;
//! - construct the bounded, lossy, non-blocking stderr writer;
//! - build the OTLP trace exporter and provider on top of a dedicated
//!   multi-threaded tokio runtime (only when an endpoint is configured);
//! - compose and install the global subscriber exactly once;
//! - own the resulting `WorkerGuard`, tracer provider, and tokio
//!   runtime so the application can shut down observability explicitly.

use std::io::IsTerminal;
use std::time::Duration;

use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use tracing_appender::non_blocking::{ErrorCounter, NonBlocking, WorkerGuard};
use tracing_subscriber::Layer;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;

use crate::error::{BootstrapError, FilterError, OtlpExporterError};

/// Default filter directive applied when `RUST_LOG` is not set.
pub const DEFAULT_FILTER: &str = "wkrun=info";

/// Logical service name attached to every emitted resource.
pub const SERVICE_NAME: &str = "wkrun";

/// Stable instrumentation scope used when constructing tracers.
pub const TRACER_NAME: &str = "wkrun";

/// Default OTLP export timeout.
pub const DEFAULT_OTLP_TIMEOUT: Duration = Duration::from_secs(10);

/// Default shutdown budget applied to remote tracer shutdown.
pub const DEFAULT_SHUTDOWN_BUDGET: Duration = Duration::from_secs(2);

/// Diagnostic format selector for the local non-blocking writer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticFormat {
    /// Human-readable, single-line-per-event format.
    Human,
    /// Newline-delimited JSON, one event per line.
    Json,
}

/// Optional OpenTelemetry trace export configuration.
#[derive(Clone, Debug)]
pub struct OtlpConfig {
    /// Endpoint override. When `None`, the exporter reads the standard
    /// OpenTelemetry environment variables.
    pub endpoint: Option<String>,
    /// Per-export timeout.
    pub timeout: Duration,
}

impl OtlpConfig {
    /// Detect OTLP configuration from the standard OpenTelemetry
    /// environment variables. Returns `None` when no endpoint is set, so
    /// ordinary invocations never attempt to contact a collector that
    /// the user did not request.
    pub fn detect_from_env() -> Option<Self> {
        let traces = std::env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT").ok();
        let general = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();
        Self::from_env_values(traces, general)
    }

    /// Construct an [`OtlpConfig`] from explicit environment values,
    /// following the standard precedence: traces-specific endpoint
    /// first, general OTLP endpoint second. Returns `None` when neither
    /// is configured.
    pub fn from_env_values(traces: Option<String>, general: Option<String>) -> Option<Self> {
        traces.or(general).map(|endpoint| Self {
            endpoint: Some(endpoint),
            timeout: DEFAULT_OTLP_TIMEOUT,
        })
    }
}

/// Internal observability configuration assembled before initialization.
#[derive(Clone, Debug)]
pub struct ObservabilityConfig {
    /// Runtime filter directive (e.g. `wkrun=debug`).
    pub filter: String,
    /// Selected diagnostic format.
    pub format: DiagnosticFormat,
    /// Optional OTLP trace export configuration.
    pub otlp: Option<OtlpConfig>,
    /// Budget applied to remote tracer shutdown.
    pub shutdown_budget: Duration,
}

impl ObservabilityConfig {
    /// Construct a configuration from the standard environment conventions.
    pub fn from_env() -> Self {
        Self {
            filter: std::env::var("RUST_LOG").unwrap_or_else(|_| DEFAULT_FILTER.to_string()),
            format: DiagnosticFormat::Human,
            otlp: OtlpConfig::detect_from_env(),
            shutdown_budget: DEFAULT_SHUTDOWN_BUDGET,
        }
    }

    /// Construct a configuration from explicit values, primarily for
    /// tests that must avoid mutating process-global environment.
    pub fn from_values(
        filter: String,
        otlp_traces: Option<String>,
        otlp_general: Option<String>,
    ) -> Self {
        Self {
            filter,
            format: DiagnosticFormat::Human,
            otlp: OtlpConfig::from_env_values(otlp_traces, otlp_general),
            shutdown_budget: DEFAULT_SHUTDOWN_BUDGET,
        }
    }
}

/// Non-fatal facts reported from an explicit shutdown operation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShutdownReport {
    /// True if the remote provider flush returned an error.
    pub remote_flush_failed: bool,
    /// True if the remote provider shutdown returned an error.
    pub remote_shutdown_failed: bool,
    /// Number of local diagnostic lines that were dropped because the
    /// bounded queue was saturated.
    pub dropped_local_lines: u64,
}

impl ShutdownReport {
    /// Render the report as a single sanitized line, or return `None`
    /// when the report has no degradation to report.
    pub fn degradation_message(&self) -> Option<String> {
        let mut parts: Vec<&'static str> = Vec::new();
        if self.remote_flush_failed {
            parts.push("remote flush failed");
        }
        if self.remote_shutdown_failed {
            parts.push("remote shutdown failed");
        }
        if self.dropped_local_lines > 0 {
            parts.push("local diagnostic lines were dropped");
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("; "))
        }
    }
}

/// Long-lived handle returned by [`install`]. Owns the writer guard,
/// tracer provider, and (when OTLP is enabled) the dedicated tokio
/// runtime that hosts the batch processor.
pub struct ObservabilityGuard {
    writer_guard: Option<WorkerGuard>,
    dropped_counter: Option<ErrorCounter>,
    dropped_lines_baseline: u64,
    tracer_provider: Option<opentelemetry_sdk::trace::SdkTracerProvider>,
    otel_runtime: Option<tokio::runtime::Runtime>,
    shutdown_budget: Duration,
}

impl ObservabilityGuard {
    /// Explicitly shut down observability. This method takes `self` by
    /// value so it can run at most once; subsequent drop is a safety net
    /// that performs best-effort cleanup.
    pub fn shutdown(mut self) -> ShutdownReport {
        let mut report = ShutdownReport::default();

        if let Some(provider) = self.tracer_provider.take() {
            if let Err(_err) = provider.force_flush() {
                tracing::warn!(
                    error_kind = "otlp_flush",
                    "otlp flush failed during shutdown"
                );
                report.remote_flush_failed = true;
            }
            match provider.shutdown_with_timeout(self.shutdown_budget) {
                Ok(()) => {}
                Err(_err) => {
                    tracing::warn!(error_kind = "otlp_shutdown", "otlp shutdown failed");
                    report.remote_shutdown_failed = true;
                }
            }
        }

        // Drop the dedicated tokio runtime before reading the dropped-line
        // counter so any tasks it spawned have ended.
        drop(self.otel_runtime.take());

        let total_dropped = self
            .dropped_counter
            .as_ref()
            .map(|counter| counter.dropped_lines() as u64)
            .unwrap_or(0);
        let new_dropped = total_dropped.saturating_sub(self.dropped_lines_baseline);
        report.dropped_local_lines = new_dropped;

        // Drop the writer guard last so buffered diagnostics flush.
        drop(self.writer_guard.take());

        if new_dropped > 0 {
            // Bypass the saturated writer; emit a direct stderr line.
            eprintln!("warning: {new_dropped} local diagnostic lines were dropped before flush");
        }

        report
    }
}

impl Drop for ObservabilityGuard {
    fn drop(&mut self) {
        // Safety net only — callers should invoke `shutdown` explicitly so
        // the shutdown report can be inspected.
        if let Some(provider) = self.tracer_provider.take() {
            let _ = provider.shutdown_with_timeout(self.shutdown_budget);
        }
    }
}

/// Install the global tracing subscriber using the given configuration.
///
/// Returns an [`ObservabilityGuard`] that owns every long-lived resource
/// required by the installed subscriber. The caller must keep the guard
/// alive for the entire duration of tracing use and must invoke
/// [`ObservabilityGuard::shutdown`] to perform an explicit shutdown.
pub fn install(config: ObservabilityConfig) -> Result<ObservabilityGuard, BootstrapError> {
    let (writer, writer_guard) = build_writer();
    let dropped_counter = writer.error_counter();
    let dropped_lines_baseline = dropped_counter.dropped_lines() as u64;

    let env_filter = build_filter(&config.filter)?;

    let (tracer_provider, otel_runtime) = match &config.otlp {
        Some(otlp_cfg) => match build_otlp_components(otlp_cfg) {
            Ok(components) => (Some(components.provider), Some(components.runtime)),
            Err(_err) => {
                // Sanitized fallback diagnostic; bypasses tracing because
                // the subscriber has not been installed yet.
                eprintln!("warning: otlp setup failed; continuing with local diagnostics only");
                (None, None)
            }
        },
        None => (None, None),
    };

    let subscriber: Box<dyn tracing::Subscriber + Send + Sync> = match &tracer_provider {
        Some(provider) => {
            let tracer = provider.tracer(TRACER_NAME);
            Box::new(
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(build_format_layer(&config.format, writer))
                    .with(tracing_opentelemetry::layer().with_tracer(tracer)),
            )
        }
        None => Box::new(
            tracing_subscriber::registry()
                .with(env_filter)
                .with(build_format_layer(&config.format, writer)),
        ),
    };

    install_subscriber(subscriber)?;

    Ok(ObservabilityGuard {
        writer_guard: Some(writer_guard),
        dropped_counter: Some(dropped_counter),
        dropped_lines_baseline,
        tracer_provider,
        otel_runtime,
        shutdown_budget: config.shutdown_budget,
    })
}

fn install_subscriber(
    subscriber: Box<dyn tracing::Subscriber + Send + Sync>,
) -> Result<(), BootstrapError> {
    let result = tracing::subscriber::set_global_default(subscriber);
    if let Err(err) = result {
        let message = err.to_string();
        if message.contains("already") || message.contains("subscriber already") {
            return Err(BootstrapError::SubscriberAlreadyInstalled);
        }
        return Err(BootstrapError::InstallSubscriber(message));
    }
    Ok(())
}

fn build_writer() -> (NonBlocking, WorkerGuard) {
    let pair = tracing_appender::non_blocking(std::io::stderr());
    (pair.0, pair.1)
}

fn build_filter(directive: &str) -> Result<EnvFilter, BootstrapError> {
    EnvFilter::try_new(directive).map_err(|err| BootstrapError::BuildFilter(FilterError(err)))
}

fn build_format_layer<S>(
    format: &DiagnosticFormat,
    writer: NonBlocking,
) -> Box<dyn Layer<S> + Send + Sync + 'static>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    let ansi = std::io::stderr().is_terminal();
    match format {
        DiagnosticFormat::Human => Box::new(
            tracing_subscriber::fmt::layer()
                .with_writer(writer)
                .with_ansi(ansi)
                .with_target(true)
                .with_level(true)
                .with_thread_ids(false)
                .with_thread_names(false),
        ),
        DiagnosticFormat::Json => Box::new(
            tracing_subscriber::fmt::layer()
                .with_writer(writer)
                .json()
                .with_current_span(true)
                .with_span_list(false)
                .with_target(true)
                .with_ansi(false),
        ),
    }
}

/// Owned pair of [`opentelemetry_sdk::trace::SdkTracerProvider`] and the
/// dedicated tokio runtime that hosts its batch processor. Returned by
/// [`build_otlp_components_with_exporter`] for use both by the production
/// install path and by integration tests.
pub struct OtlpComponents {
    provider: opentelemetry_sdk::trace::SdkTracerProvider,
    runtime: tokio::runtime::Runtime,
}

impl OtlpComponents {
    /// Borrow the tracer provider (used to install the OpenTelemetry layer).
    pub fn provider(&self) -> &opentelemetry_sdk::trace::SdkTracerProvider {
        &self.provider
    }

    /// Consume the components and return the owned tracer provider,
    /// leaving the runtime to be dropped by the caller.
    pub fn into_provider(self) -> opentelemetry_sdk::trace::SdkTracerProvider {
        self.provider
    }

    /// Drop the components, releasing the runtime and provider.
    pub fn shutdown(self) {
        drop(self.runtime);
    }
}

fn build_otlp_components(config: &OtlpConfig) -> Result<OtlpComponents, BootstrapError> {
    let mut exporter_builder = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_protocol(opentelemetry_otlp::Protocol::HttpBinary)
        .with_timeout(config.timeout);

    if let Some(endpoint) = &config.endpoint {
        exporter_builder = exporter_builder.with_endpoint(endpoint.clone());
    }

    let exporter = exporter_builder
        .build()
        .map_err(OtlpExporterError::from)
        .map_err(BootstrapError::BuildOtlpExporter)?;

    build_otlp_components_with_exporter(exporter, config)
}

/// Build OTLP components using an explicit exporter.
///
/// This function is public so that tests can inject a deterministic
/// exporter (such as the SDK's in-memory exporter) without requiring a
/// running collector. The returned `OtlpComponents` own a dedicated
/// multi-threaded tokio runtime that hosts the batch processor.
pub fn build_otlp_components_with_exporter<E>(
    exporter: E,
    _config: &OtlpConfig,
) -> Result<OtlpComponents, BootstrapError>
where
    E: opentelemetry_sdk::trace::SpanExporter + 'static,
{
    // Dedicated multi-threaded tokio runtime that hosts the OpenTelemetry
    // batch processor. We construct it ourselves so the batch processor's
    // background tasks run on a runtime we explicitly own.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .thread_name("wkrun-otel")
        .build()
        .map_err(BootstrapError::BuildOtelRuntime)?;

    let resource = opentelemetry_sdk::Resource::builder()
        .with_service_name(SERVICE_NAME)
        .with_detectors(&[
            Box::new(opentelemetry_sdk::resource::SdkProvidedResourceDetector),
            Box::new(opentelemetry_sdk::resource::EnvResourceDetector::new()),
            Box::new(opentelemetry_sdk::resource::TelemetryResourceDetector),
        ])
        .build();

    // The SDK's runtime::Tokio calls tokio::spawn, which requires an
    // entered runtime context on the current thread. We enter the
    // runtime for the duration of processor construction; the runtime
    // itself stays alive in `OtlpComponents`, so the spawned task keeps
    // running after the guard is dropped.
    let _enter = runtime.enter();
    let processor =
        opentelemetry_sdk::trace::span_processor_with_async_runtime::BatchSpanProcessor::builder(
            exporter,
            opentelemetry_sdk::runtime::Tokio,
        )
        .build();

    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_resource(resource)
        .with_span_processor(processor)
        .build();

    Ok(OtlpComponents { provider, runtime })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn shutdown_report_default_is_empty() {
        let report = ShutdownReport::default();
        assert!(report.degradation_message().is_none());
        assert_eq!(report.dropped_local_lines, 0);
        assert!(!report.remote_flush_failed);
        assert!(!report.remote_shutdown_failed);
    }

    #[test]
    fn shutdown_report_dropped_lines_produces_message() {
        let report = ShutdownReport {
            remote_flush_failed: false,
            remote_shutdown_failed: false,
            dropped_local_lines: 42,
        };
        let msg = report.degradation_message().expect("message");
        assert!(msg.contains("dropped"));
        assert!(!msg.contains("42"));
    }

    #[test]
    fn shutdown_report_combines_failures_without_secrets() {
        let report = ShutdownReport {
            remote_flush_failed: true,
            remote_shutdown_failed: false,
            dropped_local_lines: 0,
        };
        let msg = report.degradation_message().expect("message");
        assert!(msg.contains("remote flush failed"));
        assert!(!msg.contains("remote shutdown"));
    }

    #[test]
    fn config_from_env_without_overrides_uses_default_filter() {
        // Use the explicit-values constructor to avoid mutating global
        // environment, which is also consistent with the project's
        // `unsafe_code = "deny"` lint policy.
        let config = ObservabilityConfig::from_values(
            std::env::var("RUST_LOG").unwrap_or_else(|_| DEFAULT_FILTER.to_string()),
            None,
            None,
        );
        assert_eq!(config.filter, DEFAULT_FILTER);
        assert!(config.otlp.is_none());
    }

    #[test]
    fn otlp_detection_disabled_without_endpoints() {
        let detected = OtlpConfig::from_env_values(None, None);
        assert!(detected.is_none());
    }

    #[test]
    fn otlp_detection_enables_for_traces_endpoint() {
        let detected = OtlpConfig::from_env_values(Some("http://127.0.0.1:4318".to_string()), None);
        assert!(detected.is_some());
    }

    #[test]
    fn otlp_detection_prefers_traces_over_general() {
        let detected = OtlpConfig::from_env_values(
            Some("http://traces".to_string()),
            Some("http://general".to_string()),
        );
        let cfg = detected.expect("detected");
        assert_eq!(cfg.endpoint.as_deref(), Some("http://traces"));
    }

    #[test]
    fn otlp_detection_falls_back_to_general() {
        let detected = OtlpConfig::from_env_values(None, Some("http://general".to_string()));
        let cfg = detected.expect("detected");
        assert_eq!(cfg.endpoint.as_deref(), Some("http://general"));
    }

    #[test]
    fn build_filter_accepts_default_directive() {
        let result = build_filter(DEFAULT_FILTER);
        assert!(result.is_ok());
    }

    #[test]
    fn build_filter_reports_malformed_directive_without_panic() {
        let result = build_filter("==invalid==");
        assert!(result.is_err());
    }

    #[test]
    fn build_format_layer_human_does_not_panic() {
        let (writer, _guard) = build_writer();
        let _layer =
            build_format_layer::<tracing_subscriber::Registry>(&DiagnosticFormat::Human, writer);
    }

    #[test]
    fn build_format_layer_json_does_not_panic() {
        let (writer, _guard) = build_writer();
        let _layer =
            build_format_layer::<tracing_subscriber::Registry>(&DiagnosticFormat::Json, writer);
    }
}
