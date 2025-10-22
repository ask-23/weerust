## Executive Summary

This document summarizes the current state of the weewx-rs project to enable a future agent to continue execution alongside SUPERPROMPT.md and DECISIONS.md.

- Status by milestone
  - M0 (Baseline): COMPLETE
  - M1 (Ingestion & Core Pipeline): COMPLETE
  - M2 (Sinks v1): COMPLETE (HTTP API + filesystem sink). SQLite sink implemented behind feature flag.
  - M3 (Cloud Runization): COMPLETE (Dockerfile, deploy script, GitHub workflow)
  - M4 (DB & MQTT): IN PROGRESS
    - UDP ingest wired to HTTP API state (works locally)
    - HTTP ingest endpoint for Ecowitt Customized Server implemented at /ingest/ecowitt
    - SQLite sink implemented and tested (feature-gated)
    - Postgres and Influx v2 sinks added as feature-gated modules; not yet wired to ingest loop by config
    - MQTT not yet implemented
  - M5 (Hardening & Bench): NOT STARTED (initial planning only)

- Key architectural decisions (aligned with DECISIONS.md unless noted)
  - Axum 0.7 + Tokio for async HTTP server and background tasks
  - OpenTelemetry metrics via opentelemetry_prometheus for /metrics
  - Readiness is stateful, tied to internal app state
  - Modular pipeline traits (Source, Processor, Sink) in weex-core
  - Sinks are feature-gated to keep CI fast; optional Postgres/Influx left disabled by default
  - Container: distroless runtime for Cloud Run

- Critical clarification/deviation from initial assumption
  - GW1100 devices do NOT emit periodic UDP datagrams. They support:
    - Local API over TCP port 45000 (polled), and
    - HTTP “Customized Server” uploads (Ecowitt or Wunderground protocols)
  - Therefore, the current UDP INTERCEPTOR driver is not usable for GW1100 hardware. Next step is to implement an HTTP ingest endpoint compatible with Ecowitt Customized Server (recommended), or add a TCP Local API poller.

---

## Implementation Status by Milestone

### M0 – Baseline (COMPLETE)

- Implemented HTTP server with routes: /healthz, /readyz, /metrics
- Structured logging using tracing; OpenTelemetry Prometheus exporter for metrics
- Stateful readiness flag integrated with app state
- Verification:
  - cargo build/test pass
  - curl checks for /healthz, /readyz, /metrics return expected responses

### M1 – Ingestion & Core Pipeline (COMPLETE)

- Core domain types and pipeline traits in weex-core:
  - Source, Processor, Sink traits
  - WeatherPacket and ArchiveRecord types
- INTERCEPTOR UDP driver (weex-ingest) implemented with async UDP receive
- Verification:
  - Unit tests in weex-ingest
  - Integration later via CLI daemon wiring (see M4)

### M2 – Sinks v1 (COMPLETE)

- HTTP API in weewx-cli:
  - /api/v1/current returns latest WeatherPacket (200) or 204 if none
  - /api/v1/history?limit=N returns recent N packets (bounded FIFO)
- Filesystem JSONL sink (weewx-sinks::FsSink)
- SQLite sink (weewx-sinks::sqlite) implemented and feature-gated; not enabled by default
- Verification:
  - Integration test crates/weewx-cli/tests/ingest_udp.rs validates UDP → state → API

### M3 – Cloud Runization (COMPLETE)

- Multi-stage Dockerfile (Rust builder → distroless runtime)
- Cloud Run deployment script: scripts/deploy-cloudrun.sh
- GitHub Actions release workflow (created): .github/workflows/release.yml
- Verification:
  - Local container build succeeds

### M4 – DB & MQTT (IN PROGRESS)

- UDP ingest wired into daemon:
  - Background task starts InterceptorUdpDriver and injects packets into shared HTTP API state
  - Optional filesystem sink writes JSONL lines
- DB sinks (feature-gated; code present):
  - SQLite: rusqlite with bundled SQLite; creates packets table; unit-tested
  - Postgres: sqlx Postgres pool; creates packets table; insert on emit
  - InfluxDB v2: reqwest client; writes line protocol to /api/v2/write
- Wiring by config: pending
  - Next step: if configured, instantiate and append sinks (SQLite/Postgres/Influx) to ingest loop; non-fatal on errors; add tracing spans
- MQTT: pending

### M5 – Hardening & Bench (PLANNED)

- To be implemented:
  - Tracing spans and structured logs across ingest and API
  - Fuzz/property tests for parsing and serialization
  - Soak testing for UDP/HTTP ingest
  - Profiling hooks and documentation polish
  - SIGHUP config reload (where feasible)

---

## Critical Technical Discoveries

- GW1100 protocol clarification
  - The GW1100 does not broadcast periodic UDP datagrams for observation data
  - Supported modes:
    - Local API over TCP: device listens on port 45000; clients poll
    - HTTP “Customized Server” push: Ecowitt or Wunderground protocol (HTTP GET/POST to a configured URL)
- Impact:
  - The existing UDP INTERCEPTOR driver cannot be used with GW1100 hardware
- Recommended next step:
  - Implement HTTP ingest endpoints in the daemon for Ecowitt Customized Server uploads (primary), optionally WU format
  - Alternative: implement a Local API poller (TCP:45000) source for LAN deployments (not Cloud Run compatible without a proxy)

---

## Codebase Structure

- Workspace crates
  - weex-core: Domain types (WeatherPacket, ArchiveRecord), pipeline traits (Source, Processor, Sink)
  - weex-ingest: Station drivers (INTERCEPTOR UDP driver implemented)
  - weewx-cli: Main daemon and HTTP API (/healthz, /readyz, /metrics, /api/v1/*), background ingest wiring
  - weewx-config: App configuration loading; helpers for bind addresses and sink parameters
  - weewx-obs: Logging/tracing init
  - weewx-sinks: Sink implementations
    - FsSink (JSONL), Sqlite (feature: sqlite), Postgres (feature: postgres), Influx (feature: influx)
  - weex-daemon: legacy tests gated behind feature (legacy_golden) and #[ignore]

- Key modules
  - weex-core/src/pipeline.rs: Source/Processor/Sink traits
  - weex-core/src/types.rs: WeatherPacket, ObservationValue, ArchiveRecord
  - weex-ingest/src/interceptor.rs: UDP driver implementation
  - weewx-cli/src/lib.rs: HTTP routes, app state, ingest task, injection helpers

- Feature flags and optional deps
  - weewx-sinks features:
    - sqlite → rusqlite (bundled)
    - postgres → sqlx (postgres, runtime-tokio-rustls, macros)
    - influx → reqwest (rustls-tls, json)
  - Tests: legacy_golden feature to gate old golden tests

---

## Configuration Schema

- config.example.toml (key sections)
  - [station]
  - [sinks.http]
    - bind = "0.0.0.0:8080"
  - [sinks.fs]
    - dir = "/var/lib/weewx" (optional)
  - [sinks.sqlite] (feature: sqlite)
    - path = "/var/lib/weewx/weewx.db" (optional)
  - [sinks.postgres] (feature: postgres)
    - url = "postgres://user:pass@localhost:5432/weewx" (optional)
  - [sinks.influx] (feature: influx)
    - url, org, bucket, token (all required when used)
  - [ingest.interceptor]
    - bind = "0.0.0.0:9999" (default; UDP only — not applicable to GW1100)

- Environment variables: 12-factor approach is planned; config loader present; env overlay limited (future expansion acceptable)

- Wired vs planned
  - Wired: HTTP API server; UDP ingest; optional filesystem sink
  - Implemented but not yet wired by config: Postgres/Influx sinks
  - Planned: HTTP ingest endpoints for Ecowitt/WU; MQTT pub/sub; SIGHUP reload

---

## Testing & Verification

- Current test status
  - cargo test --workspace: PASS
  - Integration: crates/weewx-cli/tests/ingest_udp.rs verifies UDP → API
  - Legacy golden tests (weex-daemon): gated by feature legacy_golden and #[ignore]; default CI remains green

- How to run locally
  - make dev (runs RUST_LOG=info cargo run -p weewx-cli)

- Verify endpoints
  - curl <http://localhost:8080/healthz>
  - curl <http://localhost:8080/readyz>
  - curl <http://localhost:8080/metrics>
  - curl <http://localhost:8080/api/v1/current>
  - curl "<http://localhost:8080/api/v1/history?limit=10>"

- Known issues/gates
  - GW1100 not supported via UDP; needs HTTP ingest or TCP poller
  - DB sinks are feature-gated and not enabled by default

---

## Immediate Next Steps for Continuation

1) Implement HTTP ingest endpoint for GW1100 (Ecowitt protocol)
   - New routes: /ingest/ecowitt (and optionally /ingest/wu)
   - Map query params to WeatherPacket; inject into state and sinks
   - Document GW1100 UI configuration

2) Wire optional DB sinks into ingest loop
   - If configured, append Sqlite/Postgres/Influx sink emit in the ingest path
   - Non-fatal on sink errors; tracing spans with error context

3) Complete M4 (MQTT pub/sub)
   - Feature-gated MQTT publisher (and optional subscriber) with basic config and smoke tests

4) Begin M5 hardening
   - Add tracing spans across ingest and API
   - Property/fuzz tests for parsing/serialization
   - Soak tests for ingest; basic profiling hooks
   - SIGHUP config reload where feasible

---

## Build/Dependency Fixes Applied

- Corrected dependency name: opentelemetry-sdk → opentelemetry_sdk in crates/weewx-cli/Cargo.toml
- Ensured workspace compiles with sinks features disabled by default
- Align Axum 0.7 server pattern (tokio::net::TcpListener + axum::serve)

---

## Hardware Testing Readiness

- Current state
  - UDP driver is functional but incompatible with GW1100 (device does not emit periodic UDP datagrams)

- Required work
  - Implement HTTP ingest for Ecowitt Customized Server uploads (preferred), or TCP Local API poller on port 45000 (LAN only)

- GW1100 UI entries once HTTP ingest is ready (Ecowitt)
  - Protocol: Ecowitt (Customized Server)
  - Server/IP: IP of weewx-rs host
  - Port: 8080 (or configured HTTP port)
  - Path: /ingest/ecowitt
  - Update interval: 5s (testing), 10–30s (production)
