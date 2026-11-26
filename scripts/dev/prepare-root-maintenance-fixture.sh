#!/usr/bin/env bash
# Give the production Root a distinct upgrade identity without changing any exports.
set -euo pipefail
cp -- "$1" "$2"
# Wasm custom section, 12 payload bytes: 11-byte name "maintenance", no data.
printf '\000\014\013maintenance' >> "$2"
