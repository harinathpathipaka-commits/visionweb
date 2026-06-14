//! Prometheus-compatible metrics for observability.
//!
//! Lightweight in-process metrics using atomics — no external
//! crate dependency. Exposed at `GET /api/v1/metrics` in
//! Prometheus text exposition format.

use std::fmt::Write as FmtWrite;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::time::Instant;

/// Global metrics store, zero-alloc after init.
pub struct Metrics {
    pub sessions_active: AtomicI64,
    pub actions_total: AtomicU64,
    pub goals_active: AtomicI64,
    pub screenshots_total: AtomicU64,
    pub errors_total: AtomicU64,
    pub dom_requests_total: AtomicU64,
    pub immune_scans_total: AtomicU64,
    pub decisions_stored: AtomicU64,
    start_time: Instant,
}

impl Metrics {
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions_active: AtomicI64::new(0),
            actions_total: AtomicU64::new(0),
            goals_active: AtomicI64::new(0),
            screenshots_total: AtomicU64::new(0),
            errors_total: AtomicU64::new(0),
            dom_requests_total: AtomicU64::new(0),
            immune_scans_total: AtomicU64::new(0),
            decisions_stored: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    pub fn inc_sessions(&self, delta: i64) {
        self.sessions_active.fetch_add(delta, Ordering::Relaxed);
    }

    pub fn inc_actions(&self) {
        self.actions_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_goals(&self, delta: i64) {
        self.goals_active.fetch_add(delta, Ordering::Relaxed);
    }

    pub fn inc_screenshots(&self) {
        self.screenshots_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_errors(&self) {
        self.errors_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_dom_requests(&self) {
        self.dom_requests_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_immune_scans(&self) {
        self.immune_scans_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_decisions(&self) {
        self.decisions_stored.fetch_add(1, Ordering::Relaxed);
    }

    #[must_use]
    pub fn uptime_seconds(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    /// Render all metrics in Prometheus text exposition format.
    #[must_use]
    pub fn render(&self) -> String {
        let mut buf = String::with_capacity(1024);
        let uptime = self.uptime_seconds();

        gauge(&mut buf, "ans_uptime_seconds", uptime, "Daemon uptime");
        gauge(
            &mut buf,
            "ans_sessions_active",
            self.sessions_active.load(Ordering::Relaxed) as f64,
            "Active browser sessions",
        );
        counter(
            &mut buf,
            "ans_actions_total",
            self.actions_total.load(Ordering::Relaxed) as f64,
            "Total actions executed",
        );
        gauge(
            &mut buf,
            "ans_goals_active",
            self.goals_active.load(Ordering::Relaxed) as f64,
            "Active goals",
        );
        counter(
            &mut buf,
            "ans_screenshots_total",
            self.screenshots_total.load(Ordering::Relaxed) as f64,
            "Total screenshots captured",
        );
        counter(
            &mut buf,
            "ans_errors_total",
            self.errors_total.load(Ordering::Relaxed) as f64,
            "Total errors",
        );
        counter(
            &mut buf,
            "ans_dom_requests_total",
            self.dom_requests_total.load(Ordering::Relaxed) as f64,
            "Total DOM distillation requests",
        );
        counter(
            &mut buf,
            "ans_immune_scans_total",
            self.immune_scans_total.load(Ordering::Relaxed) as f64,
            "Total immune system scans",
        );
        counter(
            &mut buf,
            "ans_decisions_stored_total",
            self.decisions_stored.load(Ordering::Relaxed) as f64,
            "Total decision records stored",
        );

        buf
    }
}

fn gauge(buf: &mut String, name: &str, value: impl Into<f64>, help: &str) {
    write_metric(buf, name, "gauge", value, help);
}

fn counter(buf: &mut String, name: &str, value: impl Into<f64>, help: &str) {
    write_metric(buf, name, "counter", value, help);
}

fn write_metric(buf: &mut String, name: &str, mtype: &str, value: impl Into<f64>, help: &str) {
    let _ = writeln!(buf, "# HELP {name} {help}");
    let _ = writeln!(buf, "# TYPE {name} {mtype}");
    let _ = writeln!(buf, "{name} {}", value.into());
    let _ = buf.write_char('\n');
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}
