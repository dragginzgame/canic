#!/usr/bin/env bash
set -euo pipefail

# Reference canisters built for local CI/demo flows. Keep `root` last so the
# ordinary release set exists before thin-root manifest emission runs.
REFERENCE_CANISTERS=(
    app
    minimal
    user_hub
    user_shard
    scale_hub
    scale
    root
)

# Ordinary release roles expected to be staged into `root` for the reference
# topology. This intentionally excludes the hidden bootstrap `wasm_store`.
ROOT_RELEASE_SET_CANISTERS=(
    app
    minimal
    user_hub
    user_shard
    scale_hub
    scale
)
