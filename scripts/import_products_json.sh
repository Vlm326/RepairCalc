#!/bin/sh
set -eu
project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
if [ "$#" -eq 0 ]; then
    set -- "$project_root/products_final.json"
fi
exec cargo run --release --manifest-path "$project_root/parser/Cargo.toml" --bin import-products-json -- "$@"
