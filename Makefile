# Build and run GECKO locally. See AGENTS.md for the individual commands.
.PHONY: all rust wasm web test serve clean overnight overnight-dry

all: rust wasm web

rust:
	cargo build --release

wasm:
	cd web && npm run wasm

web:
	cd web && npm install --no-audit --no-fund && npm run build

test:
	cargo test --release
	cargo clippy --all-targets -- -D warnings
	cd web && npm run typecheck && npm test

serve: all
	./target/release/gecko serve

# Unattended issue queue (scripts/overnight.sh). Keeps the machine awake, runs in tmux:
#   make overnight            then detach with Ctrl-b d; reattach: tmux attach -t gecko-overnight
#   make overnight-dry        show the queue only
overnight:
	systemd-inhibit --what=idle:sleep:handle-lid-switch --why="GECKO overnight agent queue" \
		tmux new-session -s gecko-overnight "./scripts/overnight.sh; exec $$SHELL"

overnight-dry:
	./scripts/overnight.sh --dry-run
