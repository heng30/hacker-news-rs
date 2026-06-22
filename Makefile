#!/usr/bin/env bash

all: build

build:
	cargo leptos build --release

build-debug:
	cargo leptos build

build-static-linux: build
	CC_x86_64_unknown_linux_musl=x86_64-unknown-linux-musl-gcc cargo build --release --no-default-features --features ssr --target x86_64-unknown-linux-musl

debug: build-debug
	RUST_LOG=debug cargo leptos serve

watch:
	cargo leptos watch

clean:
	cargo clean

check:
	cargo check

install-linux: build-static-linux
	cp ./target/x86_64-unknown-linux-musl/release/hns ~/.local/bin/
