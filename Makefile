BINARY  := onley
APP_DIR := cmd/onley-app

.PHONY: all build app install test lint fmt clean

all: build

build:
	cargo build --release -p onley
	cp target/release/onley $(BINARY)

# Packages the macOS app with NSH. The other desktops build on their own
# systems; see .github/workflows/release.yml.
app:
	nsh package --target macos -o dist $(APP_DIR)
	@echo "Built dist/Onley.app"

# `make install` replaces the app at $(INSTALL_DIR)/Onley.app with a fresh
# package, so a local change can be tested without copying the bundle by hand.
INSTALL_DIR ?= /Applications
install: app
	@test "$$(uname -s)" = "Darwin" || { echo "make install installs the macOS app"; exit 1; }
	@mkdir -p "$(INSTALL_DIR)"
	@test -d "dist/Onley.app" || { echo "no app bundle"; exit 1; }
	@rm -rf "$(INSTALL_DIR)/Onley.app"
	@cp -R "dist/Onley.app" "$(INSTALL_DIR)/Onley.app"
	@echo "Installed $(INSTALL_DIR)/Onley.app"

test:
	cargo test --workspace

lint:
	cargo fmt --check
	cargo clippy --workspace --all-targets -- -D warnings

fmt:
	cargo fmt

clean:
	cargo clean
	rm -f $(BINARY)
	rm -rf "dist" "$(APP_DIR)/.nsh"
