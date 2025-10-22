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



# Docker build (multi-arch) using buildx. Requires DOCKER_BUILDKIT=1 and buildx.
docker-build:
	docker buildx build --platform linux/amd64,linux/arm64 -t weewx-rs:local --load .

docker-image:
	docker build -t weewx-rs:local .
