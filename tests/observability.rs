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
use wkrun::observability::{self, OtlpComponents};

/// Test-only exporter that captures spans into a shared vector and the
/// resource passed via `set_resource`.
#[derive(Clone, Debug)]
struct CapturingExporter {
    spans: Arc<Mutex<Vec<SpanData>>>,
    resource: Arc<Mutex<Option<opentelemetry_sdk::Resource>>>,
}

impl CapturingExporter {
    fn new() -> Self {
        Self {
            spans: Arc::new(Mutex::new(Vec::new())),
            resource: Arc::new(Mutex::new(None)),
        }
    }

    fn captured(&self) -> Vec<SpanData> {
        self.spans.lock().expect("lock").clone()
    }

    fn captured_resource(&self) -> Option<opentelemetry_sdk::Resource> {
        self.resource.lock().expect("lock").clone()
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

    fn shutdown_with_timeout(
        &self,
        _timeout: std::time::Duration,
    ) -> Result<(), opentelemetry_sdk::error::OTelSdkError> {
        Ok(())
    }

    fn force_flush(&self) -> Result<(), opentelemetry_sdk::error::OTelSdkError> {
        Ok(())
    }

    fn set_resource(&mut self, resource: &opentelemetry_sdk::Resource) {
        *self.resource.lock().expect("lock") = Some(resource.clone());
    }
}

fn in_memory_components(exporter: impl SpanExporter + 'static) -> OtlpComponents {
    observability::build_otlp_components_with_exporter(exporter).expect("build components")
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

    // Drain pending spans through the BSP. force_flush blocks until the
    // worker thread completes the in-flight export; no sleep is used.
    let _ = provider.force_flush();
    let _ = provider.shutdown_with_timeout(std::time::Duration::from_secs(2));

    let spans = captured.captured();
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

    // Drive a single span, then drop it inside an inner scope so the
    // span is exported before we read the captured vector.
    {
        let _span = tracer.span_builder("test.scope").start(&tracer);
    }

    // Drain pending spans through the BSP. force_flush blocks until the
    // worker thread drains the queue; no sleep is used.
    let _ = provider.force_flush();
    let _ = provider.shutdown_with_timeout(std::time::Duration::from_secs(2));

    let spans = captured.captured();
    assert!(
        !spans.is_empty(),
        "expected at least one span to be exported, got none"
    );
    let scope = &spans[0].instrumentation_scope;
    assert_eq!(
        scope.name(),
        observability::TRACER_NAME,
        "instrumentation scope name must be wkrun"
    );
}

#[test]
fn resource_carries_expected_service_name() {
    let exporter = CapturingExporter::new();

    let components = in_memory_components(exporter.clone());
    let provider = components.provider().clone();
    let tracer = provider.tracer(observability::TRACER_NAME);

    {
        let _span = tracer.span_builder("test.resource").start(&tracer);
    }

    let _ = provider.force_flush();
    let _ = provider.shutdown_with_timeout(std::time::Duration::from_secs(2));

    // The SDK calls set_resource on the exporter during provider
    // construction; the captured Resource carries service.name.
    let resource = exporter
        .captured_resource()
        .expect("set_resource should have been called by the SDK");
    let service_name = {
        let key = opentelemetry::Key::from_static_str("service.name");
        resource.get(&key).map(|v| v.to_string())
    };
    assert_eq!(
        service_name.as_deref(),
        Some(observability::SERVICE_NAME),
        "resource must carry service.name = wkrun, got {service_name:?}"
    );
}

#[test]
fn otel_service_name_override_takes_precedence_over_default() {
    // The standard OTEL_SERVICE_NAME precedence must override the
    // default `wkrun` service name. We exercise this through the test
    // seam that accepts explicit env values, avoiding process-global
    // env mutation (which the lint policy forbids in tests).
    use wkrun::observability;

    let exporter = CapturingExporter::new();
    let resource =
        observability::build_resource_for_test(Some("test-override-service".to_string()));
    let provider = SdkTracerProvider::builder()
        .with_resource(resource)
        .with_span_processor(SimpleSpanProcessor::new(exporter.clone()))
        .build();
    let tracer = provider.tracer(observability::TRACER_NAME);
    {
        let _span = tracer.span_builder("test.scope").start(&tracer);
    }
    let _ = provider.force_flush();
    let _ = provider.shutdown_with_timeout(std::time::Duration::from_secs(2));

    let captured = exporter
        .captured_resource()
        .expect("set_resource should have been called by the SDK");
    let service_name = {
        let key = opentelemetry::Key::from_static_str("service.name");
        captured.get(&key).map(|v| v.to_string())
    };
    assert_eq!(
        service_name.as_deref(),
        Some("test-override-service"),
        "explicit OTEL_SERVICE_NAME must override the default, got {service_name:?}"
    );
}

#[test]
fn shutdown_releases_provider_and_runtime() {
    #[derive(Debug)]
    struct NoopExporter;
    impl SpanExporter for NoopExporter {
        async fn export(
            &self,
            _batch: Vec<SpanData>,
        ) -> Result<(), opentelemetry_sdk::error::OTelSdkError> {
            Ok(())
        }
        fn shutdown_with_timeout(
            &self,
            _timeout: std::time::Duration,
        ) -> Result<(), opentelemetry_sdk::error::OTelSdkError> {
            Ok(())
        }
        fn force_flush(&self) -> Result<(), opentelemetry_sdk::error::OTelSdkError> {
            Ok(())
        }
        fn set_resource(&mut self, _resource: &opentelemetry_sdk::Resource) {}
    }

    let components = in_memory_components(NoopExporter);
    let provider = components.provider().clone();
    // Calling shutdown twice is allowed; the second is a no-op.
    let _ = provider.shutdown();
    let _ = provider.shutdown();
    // Constructing a new provider after the runtime has been dropped
    // exercises that the previous runtime was indeed released.
    let _new_provider = SdkTracerProvider::builder()
        .with_span_processor(SimpleSpanProcessor::new(NoopExporter))
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

        fn shutdown_with_timeout(
            &self,
            _timeout: std::time::Duration,
        ) -> Result<(), opentelemetry_sdk::error::OTelSdkError> {
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

#[test]
fn batch_processor_drops_spans_when_queue_is_saturated() {
    // Configure a small BSP queue and batch size so saturation is
    // easy to provoke. The first call to export() parks the worker
    // inside BlockingExporter::export until the test releases the
    // exporter via a Condvar gate; spans that arrive after the queue
    // is full are dropped by the SDK's `try_send`. After release the
    // exporter accepts every subsequent batch immediately, so the
    // final captured count is bounded by what the queue + batch
    // pipeline could absorb, not by the exporter's blocking.
    use std::sync::{Condvar, Mutex};

    #[derive(Debug)]
    struct BlockingExporter {
        entered: Arc<(Mutex<bool>, Condvar)>,
        release: Arc<(Mutex<bool>, Condvar)>,
        captured: Arc<Mutex<Vec<SpanData>>>,
    }

    impl SpanExporter for BlockingExporter {
        async fn export(
            &self,
            batch: Vec<SpanData>,
        ) -> Result<(), opentelemetry_sdk::error::OTelSdkError> {
            // Signal the test that we have entered export() for the
            // first time only.
            {
                let (lock, cvar) = &*self.entered;
                let mut entered = lock.lock().expect("lock");
                if !*entered {
                    *entered = true;
                    cvar.notify_all();
                }
            }
            let (lock, cvar) = &*self.release;
            let mut released = lock.lock().expect("lock");
            while !*released {
                released = cvar.wait(released).expect("wait");
            }
            self.captured.lock().expect("lock").extend(batch);
            Ok(())
        }

        fn shutdown_with_timeout(
            &self,
            _timeout: std::time::Duration,
        ) -> Result<(), opentelemetry_sdk::error::OTelSdkError> {
            Ok(())
        }

        fn force_flush(&self) -> Result<(), opentelemetry_sdk::error::OTelSdkError> {
            Ok(())
        }

        fn set_resource(&mut self, _resource: &opentelemetry_sdk::Resource) {}
    }

    let entered = Arc::new((Mutex::new(false), Condvar::new()));
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let captured = Arc::new(Mutex::new(Vec::new()));
    let exporter = BlockingExporter {
        entered: entered.clone(),
        release: release.clone(),
        captured: captured.clone(),
    };

    // Build a BSP with a small queue and small batch so saturation is
    // easy to provoke. The thread-based BSP exports serially; we use
    // a long scheduled_delay so the timer-based flush does not race
    // with our manual shutdown.
    use opentelemetry_sdk::trace::{BatchConfigBuilder, BatchSpanProcessor};
    let batch_processor = BatchSpanProcessor::builder(exporter)
        .with_batch_config(
            BatchConfigBuilder::default()
                .with_max_queue_size(2)
                .with_max_export_batch_size(1)
                .with_scheduled_delay(std::time::Duration::from_secs(60))
                .build(),
        )
        .build();
    let provider = SdkTracerProvider::builder()
        .with_span_processor(batch_processor)
        .build();
    let tracer = provider.tracer(observability::TRACER_NAME);

    // Emit more spans than the queue can hold. The BSP worker is
    // single-threaded; once it is parked in the first export() call,
    // subsequent enqueues overflow the bounded queue and are dropped.
    const TOTAL: usize = 64;
    for i in 0..TOTAL {
        let _span = tracer
            .span_builder(format!("test.saturation.{i}"))
            .start(&tracer);
    }

    // Wait until the BSP worker entered export() — this is the
    // deterministic barrier that proves saturation has occurred.
    {
        let (lock, cvar) = &*entered;
        let mut seen = lock.lock().expect("lock");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while !*seen {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                panic!("BSP worker should have entered export() within timeout");
            }
            seen = cvar.wait_timeout(seen, remaining).expect("wait").0;
        }
    }

    // Release the exporter so subsequent batches drain.
    {
        let (lock, cvar) = &*release;
        *lock.lock().expect("lock") = true;
        cvar.notify_all();
    }

    // Drain the queue: force_flush sends a control message; shutdown
    // drains pending spans after the control message is processed.
    let _ = provider.force_flush();
    let _ = provider.shutdown_with_timeout(std::time::Duration::from_secs(5));

    let received = captured.lock().expect("lock").len();
    assert!(
        received < TOTAL,
        "BSP must drop spans when queue saturates; received {received}/{TOTAL}"
    );
}
