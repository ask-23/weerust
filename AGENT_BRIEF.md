# weewx-rs Agent Brief

## Context

Rust rewrite of WeeWX, partially complete. Currently building out ingestion → processing → file/HTTP sinks.
Python code retired; Cloud Run target confirmed.

## Current focus

Finish UDP → Normalize → Files sink → `/api/v1/current` endpoint.
Everything else frozen.

## Non-negotiables

- Rust stable + Tokio
- Distroless container
- 12-factor config
- OTel metrics/logs
- Non-root Cloud Run

## What’s done

- Crate structure in place
- Config loader working
- Health endpoints stubbed
- Basic unit tests pass

## What’s next

- Implement UDP Source + pipeline linkage
- Produce JSON current output + Prometheus metrics
