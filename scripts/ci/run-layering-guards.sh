#!/usr/bin/env bash
set -euo pipefail

if rg "storage::.*Record" crates/canic-core/src/workflow --glob '!**/tests.rs'; then
    echo "workflow must not touch storage records" >&2
    exit 1
fi

if rg "(^|[^A-Za-z0-9_])api::|crate::api::" crates/canic-core/src/workflow --glob '*.rs'; then
    echo "workflow must not depend on the api layer" >&2
    exit 1
fi

if rg "struct .*Policy|enum .*Policy|impl .*Policy" crates/canic-core/src/workflow --glob '*.rs' --glob '!**/tests.rs'; then
    echo "workflow must apply policy, not define policy types" >&2
    exit 1
fi

if find crates/canic-core/src/access -name '*.rs' -print0 \
    | xargs -0 awk 'FNR == 1 { in_test = 0 } /^#\[cfg\(test\)\]/ { in_test = 1 } !in_test { print FILENAME ":" FNR ":" $0 }' \
    | rg "stable::|storage::.*Record|AppMode|EnvRecord|AppStateRecord"; then
    echo "access must use ops boundaries, not storage records or stable types" >&2
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
