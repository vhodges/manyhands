#!/bin/sh
# With no features, the package's normal dependency graph is the CLI's.
set -eu
out=$(mktemp)
trap 'rm -f "$out"' EXIT
cargo tree --locked -e normal --no-default-features -p manyhands --prefix none > "$out"
if grep -E '^gpui' "$out"; then
  echo "the no-feature build depends on a GPUI package" >&2
  exit 1
fi
