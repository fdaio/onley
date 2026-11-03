BINARY  := onley
CMD     := ./cmd/onley
GOFLAGS := -trimpath
APP_TARGET := macos_$(shell [ "$$(go env GOARCH)" = "arm64" ] && echo arm64 || echo x64)
APP_VERSION := $(shell sed -n 's/^version: //p' cmd/onley-app/nsgo.yaml)
APP_BUNDLE  := cmd/onley-app/dist/Onley-$(APP_VERSION)-$(APP_TARGET)/Onley.app
APP_BUILD   = go tool nsgo build -target $(APP_TARGET) ./cmd/onley-app

# The Homebrew cask installs to the same place; brew reinstall --cask onley
# puts that copy back.
INSTALL_DIR ?= /Applications

.PHONY: all build app app-all install test coverage lint clean

all: build

build:
	go build $(GOFLAGS) -o $(BINARY) $(CMD)

app:
	$(APP_BUILD)

app-all:
	go tool nsgo build -target macos_arm64,macos_x64,windows_x64,linux_x64 ./cmd/onley-app

# install replaces the app at $(INSTALL_DIR)/Onley.app with a fresh build, so a
# local change can be tested without copying the bundle by hand. The bundle is
# removed first because NSGo reuses an existing dist build.
install:
	@test "$$(uname -s)" = "Darwin" || { echo "make install installs the macOS app; use make app-all elsewhere"; exit 1; }
	rm -rf cmd/onley-app/dist/Onley-$(APP_VERSION)-$(APP_TARGET)
	$(APP_BUILD)
	@test -d "$(APP_BUNDLE)" || { echo "no app bundle at $(APP_BUNDLE)"; exit 1; }
	@mkdir -p "$(INSTALL_DIR)"
	@rm -rf "$(INSTALL_DIR)/Onley.app"
	@cp -R "$(APP_BUNDLE)" "$(INSTALL_DIR)/Onley.app"
	@echo "Installed $(INSTALL_DIR)/Onley.app $(APP_VERSION)"

test:
	go test ./... -count=1

coverage:
	go test ./... -count=1 -coverprofile=coverage.out
	go tool cover -func=coverage.out
	go tool cover -html=coverage.out -o coverage.html

lint:
	golangci-lint run ./...

clean:
	rm -f $(BINARY) coverage.out coverage.html
	rm -rf cmd/onley-app/dist cmd/onley-app/.nsgo
