# Build and run GECKO locally. See AGENTS.md for the individual commands.
.PHONY: all rust wasm web test serve clean overnight overnight-dry

all: rust wasm web

rust:
	cargo build --release

wasm:
	cd web && npm run wasm

web:
	cd web && npm run build

test:
	cargo test --release
	cargo clippy --all-targets -- -D warnings
	cd web && npm run test:parity
	cd web && npm run typecheck && npm test

serve: all
	./target/release/gecko serve
