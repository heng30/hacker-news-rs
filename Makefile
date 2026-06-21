#!/usr/bin/env bash

all: build

build:
	cargo build --release

build-debug:
	cargo build

build-static-linux:
	CC_x86_64_unknown_linux_musl=x86_64-unknown-linux-musl-gcc cargo build --release --target x86_64-unknown-linux-musl

debug:
	RUST_LOG=debug cargo run

debug-target: build-debug
	cd target/debug && RUST_LOG=info ./hacker-news-rs --port 3000

run:
	cargo run --release

clean:
	cargo clean

check:
	cargo check

install-linux: build-static-linux
	cp ./target/x86_64-unknown-linux-musl/release/hacker-news-rs ~/.local/bin/
