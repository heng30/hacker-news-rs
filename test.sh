#!/usr/bin/env bash

RUST_LOG=debug ./target/x86_64-unknown-linux-musl/release/hns --port 5000 --socks5-proxy socks5://127.0.0.1:1084 --search-keywords "rust,linux"

