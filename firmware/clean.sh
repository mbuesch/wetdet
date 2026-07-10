#!/bin/sh
set -e
basedir="$(dirname "$(realpath "$0")")"
cd "$basedir"
cargo clean
rm -rf ".embuild"
