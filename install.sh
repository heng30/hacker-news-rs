#!/usr/bin/env bash

TARGET_BIN_DIR="$HOME/.local/bin"

./bundle.sh

rm -rf "$TARGET_BIN_DIR"/hacker-news-dist "$TARGET_BIN_DIR"/hacker-news
cp -rf ./hacker-news "$TARGET_BIN_DIR"/hacker-news
cp -rf ./dist "$TARGET_BIN_DIR"/hacker-news-dist

