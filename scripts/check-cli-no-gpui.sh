#!/bin/sh
# With no features, the package's normal dependency graph is the CLI's.
# It holds no GPUI package, and none of the desktop's editor crates: the
# editor (`zorite…`), its Markdown parser (`markdown`) and the image
# decoder (`image`).
set -eu
out=$(mktemp)
trap 'rm -f "$out"' EXIT
cargo tree --locked -e normal --no-default-features -p manyhands --prefix none > "$out"
if grep -E '^gpui' "$out"; then
  echo "the no-feature build depends on a GPUI package" >&2
  exit 1
fi
if grep -E '^zorite[^ ]* |^(image|markdown) ' "$out"; then
  echo "the no-feature build depends on a desktop editor crate" >&2
  exit 1
fi
