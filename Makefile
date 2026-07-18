.PHONY: test clippy fmt ci build install uninstall ui

test:
	cargo test --workspace

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

fmt:
	cargo fmt --all

ci: fmt
	cargo fmt --all --check
	$(MAKE) clippy
	$(MAKE) test
	node --test adapters/cursor/hooks/microbridge-event.test.mjs
	cd apps/microbridge-ui && npm ci && npm test && npm run build

build:
	cargo build --release -p microbridged -p microbridgectl

install:
	./scripts/install.sh

uninstall:
	./scripts/uninstall.sh

ui:
	cd apps/microbridge-ui && npm install && npm run dev
