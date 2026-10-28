BINARY  := onley
CMD     := ./cmd/onley
GOFLAGS := -trimpath
APP_TARGET := macos_$(shell [ "$$(go env GOARCH)" = "arm64" ] && echo arm64 || echo x64)

.PHONY: all build app app-all test coverage lint clean

all: build

build:
	go build $(GOFLAGS) -o $(BINARY) $(CMD)

app:
	go tool nsgo build -target $(APP_TARGET) ./cmd/onley-app

app-all:
	go tool nsgo build -target macos_arm64,macos_x64,windows_x64,linux_x64 ./cmd/onley-app

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
