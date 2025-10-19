WEEWX-RS — Rust Refactor Superprompt (Cloud Run–ready)

Purpose: Direct an autonomous coding agent to implement a robust, portable, low-overhead Rust replacement for WeeWX (“weewx-rs”), containerized for Google Cloud Run, but runnable on bare metal (NUC), Docker, and Kubernetes.
Format: Obsidian-friendly Markdown. Copy this whole file as the working spec.

0) Executive Summary

Goal: Build weewx-rs: a modular, high-performance weather station engine that ingests device data, processes/archives it, and publishes outputs (files/APIs/DBs/MQTT) with first-class observability and CI/CD to Cloud Run.

Prior art: WeeWX (Python). We are replacing with Rust for reliability, memory safety, and lower runtime cost on Cloud Run.

Key wins: zero-crash daemon, minimal image (<60MB), cold-start-friendly, configurable pipelines, batteries-included metrics/logs/traces, one-command deploys.

1) Non-Negotiables (Read First)

Language/runtime: Rust stable, async (Tokio), 2021 edition. No Python.

Container: Distroless or gcr.io/distroless/cc final image; multi-stage build; --platform=linux/amd64,linux/arm64. Static if practical (musl).

I/O: No blocking on async paths. Backpressure respected.

Config: 12-factor: env vars + optional config.{toml,yaml}; hot-reload safe (SIGHUP).

Observability: OpenTelemetry traces + metrics + structured logs; /healthz, /readyz, /metrics.

Cloud Run: Concurrency tuned, graceful shutdown (SIGTERM), request/CPU awareness, low memory (target 128–256MB).

Security: Non-root user, read-only root FS, drop caps, no shell, validated inputs, secrets via GCP Secret Manager or env.

2) Scope & Capabilities
2.1 Ingestion (Devices/Transports)

Implement a pluggable Source trait:

Serial/USB (e.g., Davis Vantage, Fine Offset, WH series)

TCP/UDP listeners (for gateways, custom sensors)

MQTT subscribe (optional)

File tail / stdin (test harness)

Backpressure + reconnect logic; device autodetect by config; robust error taxonomy.

2.2 Processing Pipeline

Stream model with stages:

Normalization: unit conversion (SI configurable), timestamp harmonization, dedupe.

Calibration: offsets, linear transforms, spike filtering.

Derived metrics: wind gust, feels-like, dew point, rainfall rate, barometer sea-level adjust.

Aggregation: 1-min, 5-min, hourly, daily windows; tumbling + session windows with watermarking.

2.3 Storage & Publishing (Sinks)

Pluggable Sink trait—enable any subset:

Filesystem exporter: static HTML, JSON, CSV; theming via templates (Tera/Askama).

APIs: HTTP/JSON and optional gRPC for realtime/current + historical queries.

DBs: SQLite (local), Postgres, InfluxDB v2 (line protocol), Prometheus (via /metrics).

MQTT publish: topic schema weewx/{station}/{metric}.

GCS uploader: optional, for static site artifacts (cache-busted).

Pub/Sub: publish normalized readings for downstream consumers.

2.4 Admin & Ops

HTTP admin UI (minimal): status, recent readings, config view (read-only), health.

CLI: weewx-rs ingest, process, serve, bench, export, doctor.

Migrations: for SQL schemas when enabled.

3) Architecture
weewx-rs
 ├─ crates/
 │   ├─ weewx-core         # Domain types, units, traits (Source, Processor, Sink), pipeline runtime
 │   ├─ weewx-sources      # Serial/TCP/MQTT/file adapters
 │   ├─ weewx-processors   # calibration, derived metrics, aggregation
 │   ├─ weewx-sinks        # fs/html, http, grpc, sqlite/postgres, influx, mqtt, pubsub, gcs
 │   ├─ weewx-config       # config loading/validation, env overlay, secrets
 │   ├─ weewx-obs          # tracing, metrics, logging, health/readiness
 │   ├─ weewx-cli          # binaries: main daemon + tools
 │   └─ weewx-themes       # default Tera templates, assets, site builder
 ├─ deploy/
 │   ├─ cloudrun/          # service.yaml, run.sh, min/max instances, CPU/throttle
 │   ├─ terraform/         # optional: Cloud Run, Pub/Sub, Secret Manager, GCS
 │   └─ k8s/               # optional manifests (HPA, PDB, NetworkPolicy)
 ├─ .github/workflows/     # CI: build/test/lint; CD: Cloud Run
 ├─ Dockerfile
 ├─ Makefile
 ├─ config.example.toml
 └─ README.md

Data model: Measurement { ts, station_id, metric: enum, value: f64, unit } with type-safe unit system.

Throughput target: ≥1k msgs/sec on x86_64, steady 128MB RSS, p50 < 5ms pipeline latency.

4) Configuration
4.1 Environment Variables

WEEWX_CONFIG=/etc/weewx/config.toml (optional)

WEEWX_STATION_ID, WEEWX_TIMEZONE

WEEWX_SOURCES=serial:/dev/ttyUSB0,baud=19200;udp:0.0.0.0:5555

WEEWX_SINKS=fs:./site;http:0.0.0.0:8080;prom:/metrics

WEEWX_DB_URL=postgres://… (optional)

OTEL_EXPORTER_OTLP_ENDPOINT, OTEL_SERVICE_NAME=weewx-rs

RUST_LOG=info,weewx=debug

4.2 config.toml (example)
[station]
id = "home"
timezone = "America/Chicago"

[sources.serial]
device = "/dev/ttyUSB0"
baud = 19200
protocol = "vantage"

[processors.calibration]
temp_offset_c = -0.2
pressure_offset_hpa = 1.1

[processors.aggregation]
windows = ["1m","5m","1h","1d"]

[sinks.files]
path = "./site"
theme = "default"

[sinks.http]
bind = "0.0.0.0:8080"

[sinks.influx]
url = "<http://influxdb:8086>"
org = "weewx"
bucket = "weather"
token = "env:INFLUX_TOKEN"

5) Cloud Run Profile

Port: 8080 (HTTP API, metrics, health)

Concurrency: start at 40; tune between 20–80

Memory: 256Mi; CPU: 1 vCPU (activate on request to save cost)

Min instances: 0 (default) for hobby; 1 for near-zero cold starts

Max instances: 5–20 (configurable)

Readiness: /readyz checks sinks initialized & sources connected (or in retry)

Liveness: /healthz lightweight self-check

Files: use /tmp only; artifacts synced to GCS when enabled

Secrets: GCP Secret Manager → env at deploy

6) Deliverables

Compilable repo with crates as above; cargo test and cargo fmt/clippy clean.

Docker images (amd64/arm64), SBOM and minimal size target (<60MB).

Makefile targets (see §12).

Cloud Run deploy script + GitHub Actions pipeline (build, test, scan, push, deploy).

Docs: Quickstart (NUC & Docker), Cloud Run guide, config reference, device matrix.

Bench suite for parsing/processing; perf report in CI.

Theme: minimal default HTML site (Tera), Lighthouse-friendly.

7) Implementation Priorities (Milestones)

M0 — Repo & Skeleton (Day 0–1)

Workspace, crates, error taxonomy, config loader, observability, health endpoints.

M1 — Ingestion & Core Pipeline (Day 2–5)

Serial + UDP source; normalization; calibration; derived metrics; /metrics.

M2 — Sinks v1 (Day 6–9)

Filesystem site export, HTTP current endpoints (/api/v1/current, /api/v1/history?range=), Prometheus.

M3 — Cloud Runization (Day 10–12)

Dockerfile multi-arch, non-root, read-only; deploy scripts; readiness/liveness; CI/CD to Cloud Run.

M4 — DB & MQTT (Day 13–16)

SQLite/Postgres, Influx v2; MQTT pub/sub; GCS site sync.

M5 — Hardening & Bench (Day 17–20)

Fuzz key parsers, property tests, soak test; memory/cpu profiling; docs polish.

8) API Contract
8.1 HTTP (JSON)

GET /api/v1/current?station=home

GET /api/v1/history?station=home&window=1m&since=…&until=…

GET /metrics (Prometheus)

GET /healthz / GET /readyz

Types are RFC3339 timestamps, SI units unless configured otherwise.

8.2 MQTT

Publish: weewx/{station}/{metric} payload JSON ({ts, value, unit})

Retain last known per metric (configurable)

9) Observability

Tracing: tracing + opentelemetry-otlp exporter.

Metrics: request latency, pipeline stage durations, queue depths, drop counts, reconnects.

Logs: JSON (Cloud Logging friendly), redaction for secrets, correlation IDs.

10) Performance & Reliability Budget

p50 end-to-end < 5ms, p99 < 50ms at 1k rps equivalent ingest.

RSS target ≤ 256MB; CPU ≤ 200m steady on Cloud Run baseline.

At-least-once delivery semantics from sources; idempotent sinks where possible.

Graceful shutdown deadline 10s; flush buffers; close spans.

11) Security & Compliance

Non-root UID/GID, read-only FS, CAP_NET_BIND_SERVICE if needed only.

Dependencies pinned; cargo deny for advisories; SLSA level in CI where possible.

Input validation for all device/parsers; time skew checks; DOS mitigation on UDP.

Secrets only via env/Secret Manager; no secrets in logs.

12) Developer Experience
12.1 Makefile Targets
make help           # list
make setup          # toolchain, git hooks
make lint           # fmt + clippy
make test           # unit/integration
make bench          # criterion benches
make dev            # run locally with config.example.toml
make docker         # build multi-arch
make sbom           # syft or cargo auditable SBOM
make run-cloudrun   # deploy/update Cloud Run
make doctor         # env checks, permissions, USB hints

12.2 GitHub Actions

ci.yml: fmt, clippy, test, bench (on label), cargo deny, build

release.yml: tag → multi-arch build → push → create GitHub Release

deploy-cloudrun.yml: on main merge; uses gcloud with workload identity

13) Dockerfile (Spec)

Stage 1: rust:1-slim builder, install musl-tools (if static), cargo-chef for layer caching.

Stage 2: gcr.io/distroless/cc or scratch (if static), copy binary to /usr/local/bin/weewx.

Set USER 65532:65532, WORKDIR /app, EXPOSE 8080, ENTRYPOINT ["/usr/local/bin/weewx"].

Drop CAP_NET_RAW; mount /tmp as writable; otherwise read-only FS.

14) Cloud Run Deployment (Spec)

Service name: weewx-rs

Region: us-central1 (override allowed)

Flags: --allow-unauthenticated (configurable), --cpu=1, --memory=256Mi, --concurrency=40

Env: from repo .env.cloudrun + Secret Manager refs

Traffic: 100% to latest by default

Rollback plan: keep last 2 revisions; fast rollback script

15) Testing Strategy

Unit tests: trait conformance, parsers, unit conversions, aggregation math.

Property tests: quickcheck/bolero on derived metrics & windows.

Integration tests: spin UDP/serial simulators, assert sink outputs & API.

Contract tests: OpenAPI schema tests for /api.

Bench: parse → process → sink micro-benches.

Soak: 24h synthetic stream; memory leak detection.

16) Compatibility Matrix (Initial)

Stations: Davis Vantage (serial), Fine Offset/WH (UDP), generic NMEA-like adapters.

Platforms: Linux x86_64/arm64, macOS (dev), Cloud Run, Docker, k8s.

DBs: SQLite (default), Postgres 13+, InfluxDB 2.x.

17) Theming & Site Export

Templates: Tera; base theme “plain-air”.

Artifacts: /site/index.html, /site/current.json, daily CSV/JSON per metric.

Accessibility: semantic HTML, no JS requirement for basics; optional progressive enhancements.

18) Risks & Mitigations

Device protocol quirks: isolate parsers per protocol; fuzz; config toggles.

Clock drift: NTP assumptions documented; detect large skews; warn.

Cold starts: keep min instance = 1 if publishing to time-sensitive endpoints.

Cost: default no-DB + filesystem only; CPU on-demand; gzip responses.

19) Nice-to-Haves (Defer)

gRPC service; WebSocket live stream; NATS JetStream; Web UI dashboards; WASM parsers.

20) Acceptance Criteria (Go/No-Go)

docker run with config.example.toml produces site files and serves /api/v1/current.

gcloud run deploy success; health passes; metrics visible; logs structured.

Sustained ingest of 10 msg/sec for 1h on min instance without errors/drops.

CI green on fmt/clippy/test/deny; image size target met; SBOM attached.

Docs let a new user deploy in <15 minutes.

21) Work Plan for the Agent

Scaffold workspace & crates; implement config + obs + health.

Build Source/Sink traits; implement UDP + Serial sources.

Implement processors (calibration, derived, aggregation).

Implement filesystem sink + HTTP API + Prometheus.

Wire CLI; add doctor checks (USB permissions).

Add Dockerfile, Makefile, GH Actions; push multi-arch.

Add Cloud Run deploy + sample terraform (optional).

Add DB/MQTT/GCS integrations.

Write docs, examples, and default theme.

Bench + harden; deliver.

22) Repository Metadata

License: MIT or Apache-2.0 (pick one, default Apache-2.0).

Code style: rustfmt default; clippy pedantic allowed with justified allows.

Conventional commits and semantic versioning.

23) Prompts & Guardrails for AI Agents

Prefer small, tested PRs. Add tests with new code.

Don’t add heavy deps; justify any > 1MB addition.

Keep public APIs minimal and documented with examples.

Treat configuration and errors as UX: great messages, hints, remediation.

24) Checklists

Pre-commit

 fmt/clippy

 unit tests

 docs updated

Pre-release

 CI green

 images built multi-arch

 changelog updated

 deploy to staging Cloud Run

 smoke tests pass

25) Quickstart Snippets

Local (NUC)

cp config.example.toml config.toml
cargo run --bin weewx -- --config ./config.toml

Docker

docker buildx build --platform linux/amd64,linux/arm64 -t ghcr.io/you/weewx-rs:dev . --push
docker run --rm -p 8080:8080 -v $PWD/site:/site ghcr.io/you/weewx-rs:dev

Cloud Run

gcloud run deploy weewx-rs \
  --source . \
  --region us-central1 \
  --allow-unauthenticated \
  --concurrency 40 \
  --cpu 1 --memory 256Mi \
  --set-env-vars WEEWX_STATION_ID=home

26) Open Questions (Track, but proceed)

Which station protocols must be first-class on day one? INTERCEPTOR ONLY!!

How much of the legacy WeeWX theming surface do we mimic? (Default: minimal)

# KEY FACTS FOR RESTART

Load AGENT_BRIEF.md for context.

Obey DECISIONS.md as immutable facts.

Return unified diffs and updated tests. No global refactors.
