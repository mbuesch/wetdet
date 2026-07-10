#!/bin/sh
set -e
basedir="$(dirname "$(realpath "$0")")"
cd "$basedir"
cargo +esp espflash flash --erase-parts nvs,phy_init,factory --monitor --release "$@"
