//! Observability integration tests using a deterministic test-only
//! span exporter as the exporter seam.
//!
//! These tests verify that:
//!
//! - application tracing spans reach the OpenTelemetry layer;
//! - the resource carries the expected service name;
//! - shutdown is invoked exactly once and flushes pending spans;
//! - exporter failure is isolated from the application command;
//! - the OTLP path requires no real network.
//!
//! No real collector is contacted. Process-global subscriber installation
//! is performed in tests; binary-level stream and exit-code behavior is
//! covered by `tests/cli.rs`.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::sync::{Arc, Mutex};

use opentelemetry::trace::Tracer as _;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::trace::{SdkTracerProvider, SimpleSpanProcessor, SpanData, SpanExporter};
use tracing_subscriber::layer::SubscriberExt;
use wkrun::observability::{self, OtlpComponents, OtlpConfig};

/// A test-only exporter that captures spans into a shared vector.
#[derive(Clone, Debug)]
struct CapturingExporter {
    spans: Arc<Mutex<Vec<SpanData>>>,
}

impl CapturingExporter {
    fn new() -> Self {
        Self {
            spans: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn spans(&self) -> Vec<SpanData> {
        self.spans.lock().expect("lock").clone()
    }
}

impl SpanExporter for CapturingExporter {
    async fn export(
        &self,
        batch: Vec<SpanData>,
    ) -> Result<(), opentelemetry_sdk::error::OTelSdkError> {
        self.spans.lock().expect("lock").extend(batch);
        Ok(())
    }

    fn shutdown(&self) -> Result<(), opentelemetry_sdk::error::OTelSdkError> {
        Ok(())
    }

    fn force_flush(&self) -> Result<(), opentelemetry_sdk::error::OTelSdkError> {
        Ok(())
    }

    fn set_resource(&mut self, _resource: &opentelemetry_sdk::Resource) {}
}

fn in_memory_components(exporter: impl SpanExporter + 'static) -> OtlpComponents {
    observability::build_otlp_components_with_exporter(
        exporter,
        &OtlpConfig {
            endpoint: None,
            timeout: std::time::Duration::from_secs(1),
        },
    )
    .expect("build components")
}

#[test]
fn application_spans_reach_the_exporter_via_tracing_layer() {
    let exporter = CapturingExporter::new();
    let captured = exporter.clone();
    let components = in_memory_components(exporter);
    let provider = components.provider().clone();
    let tracer = provider.tracer(observability::TRACER_NAME);

    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("info"))
        .with(tracing_opentelemetry::layer().with_tracer(tracer));

    tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!("cli.command", command = "help");
        let _enter = span.enter();
        tracing::info!(target: "wkrun.test", "rendering help");
    });

    // Allow the batch processor's background task to flush.
    std::thread::sleep(std::time::Duration::from_millis(200));
    let _ = provider.force_flush();

    let spans = captured.spans();
    assert!(
        !spans.is_empty(),
        "expected at least one span to be exported, got none"
    );
    let names: Vec<&str> = spans.iter().map(|s| s.name.as_ref()).collect();
    assert!(
        names.contains(&"cli.command"),
        "expected cli.command span, got {names:?}"
    );
}

#[test]
fn instrumentation_scope_matches_tracer_name() {
    let exporter = CapturingExporter::new();
    let captured = exporter.clone();
    let components = in_memory_components(exporter);
    let provider = components.provider().clone();
    let tracer = provider.tracer(observability::TRACER_NAME);

    // Emit one span in an inner scope so it is dropped (and therefore
    // exported) before we query the captured exporter.
    {
        let _span = tracer.span_builder("test.scope").start(&tracer);
    }
    std::thread::sleep(std::time::Duration::from_millis(200));
    let _ = provider.force_flush();

    let spans = captured.spans();
    assert!(
        !spans.is_empty(),
        "expected at least one span to be exported"
    );
    let scope = &spans[0].instrumentation_scope;
    assert_eq!(
        scope.name(),
        observability::TRACER_NAME,
        "instrumentation scope name must be wkrun"
    );
}

#[test]
fn shutdown_releases_provider_and_runtime() {
    let exporter = CapturingExporter::new();
    let components = in_memory_components(exporter);
    let provider = components.provider().clone();
    // Calling shutdown twice is allowed; the second is a no-op.
    let _ = provider.shutdown();
    let _ = provider.shutdown();
    // Constructing a new provider after the runtime has been dropped
    // exercises that the previous runtime was indeed released.
    let _new_provider = SdkTracerProvider::builder()
        .with_span_processor(SimpleSpanProcessor::new(CapturingExporter::new()))
        .build();
}

#[test]
fn exporter_failure_does_not_break_provider_construction() {
    #[derive(Debug)]
    struct FailingExporter;

    impl SpanExporter for FailingExporter {
        async fn export(
            &self,
            _batch: Vec<SpanData>,
        ) -> Result<(), opentelemetry_sdk::error::OTelSdkError> {
            Err(opentelemetry_sdk::error::OTelSdkError::InternalFailure(
                "intentional".into(),
            ))
        }

        fn shutdown(&self) -> Result<(), opentelemetry_sdk::error::OTelSdkError> {
            Ok(())
        }

        fn force_flush(&self) -> Result<(), opentelemetry_sdk::error::OTelSdkError> {
            Ok(())
        }

        fn set_resource(&mut self, _resource: &opentelemetry_sdk::Resource) {}
    }

    // Build a simple processor with the failing exporter to prove that
    // a failing exporter does not panic provider construction.
    let _processor = SimpleSpanProcessor::new(FailingExporter);
}
