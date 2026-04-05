#!/usr/bin/env bash
set -euo pipefail

if rg "storage::stable::.*Record" crates/canic-core/src/workflow --glob '!**/tests.rs'; then
    echo "workflow must not touch storage records" >&2
    exit 1
fi

if rg "pub use .*Record" crates/canic-core/src | rg -v "pub\\(crate\\)"; then
    echo "record types must not be publicly re-exported" >&2
    exit 1
fi

if rg "(to_view|from_view)" crates/canic-core/src | rg -v "record_to_view|view::"; then
    echo "misuse of 'view' detected in function names" >&2
    exit 1
fi
