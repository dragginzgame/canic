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
printf '%s\n' "$@" > "$SCOPE_TEST_OUTPUT/workspace.args"
[[ "${SCOPE_TEST_FAILURE:-}" != workspace ]] || exit 23
SH
chmod +x "$fixture/bin/cargo"

run() {
    PATH="$fixture/bin:$PATH" SCOPE_TEST_OUTPUT="$fixture" \
        bash "$fixture/scripts/ci/run-workspace-cargo.sh" "$@"
}

require_workspace_scope() {
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
for argument in --locked --all-targets -D warnings; do
    rg -x -- "$argument" "$fixture/workspace.args" >/dev/null
done
status=0
SCOPE_TEST_FAILURE=workspace run clippy -D warnings > "$fixture/failure.log" 2>&1 || status=$?
[[ "$status" -eq 23 ]]
echo 'workspace Cargo scope tests passed'
