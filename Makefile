all: build

build:
	cargo build --release

install:
	cargo fetch

run-server:
	cargo run --bin server -- src/assets/default_world.json

run-client:
	cargo run --bin client_cli

run-client-gui:
	cargo run --bin client_gui

lint:
	cargo check

clean:
	cargo clean

nc:
	nc 127.0.0.1 1234

.PHONY: all build install run-server run-client run-client-gui lint clean
