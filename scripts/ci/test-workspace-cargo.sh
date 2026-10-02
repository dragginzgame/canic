#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-workspace-cargo-test.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/scripts/ci" "$fixture/bin"
cp "$ROOT/scripts/ci/"{run-workspace-cargo.sh,workspace-scope.sh} \
    "$fixture/scripts/ci/"
cat > "$fixture/bin/cargo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
case " $* " in
    *' -p canic-tests '*) phase=integration ;;
    *) phase=workspace ;;
esac
printf '%s\n' "$@" > "$SCOPE_TEST_OUTPUT/$phase.args"
[[ "$phase" != "${SCOPE_TEST_FAILURE:-}" ]] || exit 23
SH
chmod +x "$fixture/bin/cargo"

run() {
    PATH="$fixture/bin:$PATH" SCOPE_TEST_OUTPUT="$fixture" \
        bash "$fixture/scripts/ci/run-workspace-cargo.sh" "$@"
}

require_workspace_scope() {
    local package
    for package in canic-icydb-lifecycle-schema canic_icydb_lifecycle_probe; do
        awk -v package="$package" '
            previous == "--exclude" && $0 == package { found = 1 }
            { previous = $0 }
            END { exit !found }
        ' "$fixture/workspace.args"
    done
    rg -x -- '--workspace' "$fixture/workspace.args" >/dev/null
    rg -x -- '--locked' "$fixture/workspace.args" >/dev/null
}

for operation in build check; do
    run "$operation" --keep-going
    require_workspace_scope
    rg -x -- '--keep-going' "$fixture/workspace.args" >/dev/null
done
run clippy -D warnings
require_workspace_scope
rg -x -- '--all-features' "$fixture/workspace.args" >/dev/null
if rg -x -- '--all-features|--features' "$fixture/integration.args"; then exit 1; fi
for phase in workspace integration; do
    rg -x -- '--locked' "$fixture/$phase.args" >/dev/null
    rg -x -- '--all-targets' "$fixture/$phase.args" >/dev/null
    rg -x -- '-D' "$fixture/$phase.args" >/dev/null
    rg -x -- 'warnings' "$fixture/$phase.args" >/dev/null
done
for phase in workspace integration; do
    status=0
    SCOPE_TEST_FAILURE="$phase" run clippy -D warnings > "$fixture/failure.log" 2>&1 || status=$?
    [[ "$status" -eq 23 ]]
done
echo 'workspace Cargo scope tests passed'
