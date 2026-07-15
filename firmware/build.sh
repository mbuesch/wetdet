#!/bin/sh
set -e
basedir="$(dirname "$(realpath "$0")")"
cd "$basedir"
cargo +esp build --package main --release
