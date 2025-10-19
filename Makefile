help:
	@echo "make help           # list"
	@echo "make lint           # fmt + clippy"
	@echo "make test           # unit/integration"
	@echo "make dev            # run locally with config.example.toml"

lint:
	cargo fmt --all
	cargo clippy --all-targets -- -D warnings

test:
	cargo test --workspace --all-features --verbose

dev:
	RUST_LOG=info cargo run -p weewx-cli

