# Build and run GECKO locally. See AGENTS.md for the individual commands.
#   make / make serve   fast dev build (no wasm-opt, no tsc) and serve
#   make release        everything, fully optimized (wasm-opt, typecheck)
.PHONY: all dev release rust wasm wasm-dev web web-dev test serve serve-release clean

all: dev

# Dev: skip wasm-opt and the separate tsc pass (use `make test` / `npm run typecheck`).
dev: rust wasm-dev web-dev

# Release: wasm-opt on the wasm bundle, typecheck before the web build.
release: rust wasm web

rust:
	cargo build --release

wasm:
	cd web && npm run wasm

wasm-dev:
	cd web && npm run wasm:dev

web:
	cd web && npm run build

web-dev:
	cd web && npm run build:dev

test:
	cargo test --release
	cargo clippy --all-targets -- -D warnings
	cd web && npm run test:parity
	cd web && npm run typecheck && npm run test:contract && npm test
	python3 -I research/scripts/research.py lint

serve: dev
	./target/release/gecko serve

serve-release: release
	./target/release/gecko serve
