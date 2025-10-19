# Scope: WeeWX parity port
- Language change only (Rust); no new features.
- Must write *exactly* the same MySQL rows as current WeeWX/WeeWX-MySQL pipeline.
- Keep existing schema; no migrations.
- TDD with golden DB diffs: identical rows, units, timestamps.
- Containerized output; target: Cloud Run.

