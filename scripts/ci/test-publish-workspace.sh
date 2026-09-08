#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FIXTURE="$(mktemp -d "${TMPDIR:-/tmp}/canic-publish-runner.XXXXXX")"
trap 'rm -rf "$FIXTURE"' EXIT
mkdir -p "$FIXTURE/scripts/ci" "$FIXTURE/bin" "$FIXTURE/registry"
cp "$ROOT/scripts/ci/publish-workspace.sh" "$FIXTURE/scripts/ci/"
printf '#!/usr/bin/env bash\necho 0.1.0\n' >"$FIXTURE/scripts/ci/read-workspace-version.sh"
printf '#!/usr/bin/env bash\nexit 0\n' >"$FIXTURE/scripts/ci/check-release-candidate.sh"
cat >"$FIXTURE/scripts/ci/check-publish-manifest-boundary.sh" <<'SH'
#!/usr/bin/env bash
exit "${FAKE_MANIFEST_STATUS:-0}"
SH
cat >"$FIXTURE/bin/cargo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$PUBLICATION_TEST_EVENTS"
case "$1" in
    info) [[ -f "$PUBLICATION_TEST_REGISTRY/${2%@*}" ]] ;;
    publish)
        [[ "$2" == -p && "$4" == --locked ]] || exit 98
        [[ "${FAKE_FAIL_PACKAGE:-}" != "$3" ]] || exit 37
        if [[ " $* " != *' --dry-run '* ]]; then
            touch "$PUBLICATION_TEST_REGISTRY/$3"
        fi
        ;;
    *) echo "unexpected Cargo command: $*" >&2; exit 99 ;;
esac
SH
chmod +x "$FIXTURE/bin/cargo"
export PUBLICATION_TEST_EVENTS="$FIXTURE/events"
export PUBLICATION_TEST_REGISTRY="$FIXTURE/registry"
export CANIC_PUBLICATION_LOG_DIR="$FIXTURE/logs"
export PUBLISH_FROM='' PUBLISH_DRY_RUN=0
expected_packages=(canic-backup canic-core canic-control-plane canic-macros canic canic-host canic-cli)

run_case() {
    local expected="$1" status=0
    PATH="$FIXTURE/bin:$PATH" PUBLISH_POLL_SECS=0 PUBLISH_TIMEOUT_SECS=1 \
        bash "$FIXTURE/scripts/ci/publish-workspace.sh" >"$FIXTURE/output.log" 2>&1 || status=$?
    if [[ "$status" -ne "$expected" ]]; then
        cat "$FIXTURE/output.log" >&2
        echo "publication fixture: expected $expected, got $status" >&2
        exit 1
    fi
}

FAKE_MANIFEST_STATUS=19 run_case 19
[[ ! -e "$PUBLICATION_TEST_EVENTS" ]]
FAKE_FAIL_PACKAGE=canic-core run_case 37
[[ -f "$PUBLICATION_TEST_REGISTRY/canic-backup" ]]
[[ ! -f "$PUBLICATION_TEST_REGISTRY/canic-core" ]]
if rg '^publish -p canic-control-plane ' "$PUBLICATION_TEST_EVENTS" >/dev/null; then
    echo 'publication continued after a package failure' >&2
    exit 1
fi
run_case 0
for package in "${expected_packages[@]}"; do
    [[ -f "$PUBLICATION_TEST_REGISTRY/$package" ]]
done
publish_calls="$(rg -c '^publish ' "$PUBLICATION_TEST_EVENTS")"
lookup_calls="$(rg -c '^info ' "$PUBLICATION_TEST_EVENTS")"
run_case 0
[[ "$(rg -c '^publish ' "$PUBLICATION_TEST_EVENTS")" == "$publish_calls" ]]
[[ "$(( $(rg -c '^info ' "$PUBLICATION_TEST_EVENTS") - lookup_calls ))" -eq "${#expected_packages[@]}" ]]

# A resumed suffix must still reject a missing predecessor package.
rm "$PUBLICATION_TEST_REGISTRY/canic-backup"
PUBLISH_FROM=canic-host run_case 1
[[ "$(rg -c '^publish ' "$PUBLICATION_TEST_EVENTS")" == "$publish_calls" ]]
touch "$PUBLICATION_TEST_REGISTRY/canic-backup"
PUBLISH_FROM=canic-host run_case 0

timings="$(rg --files "$CANIC_PUBLICATION_LOG_DIR" | rg '/timings.tsv$')"
while IFS= read -r file; do
    awk -F '\t' 'NR == 1 { if ($0 != "stage\texit_code\tseconds\tlog") exit 1; next }
        $2 !~ /^[0-9]+$/ || $3 !~ /^[0-9]+$/ { exit 1 }' "$file"
done <<<"$timings"
rg -l $'^publish-canic-core\t37\t' "$CANIC_PUBLICATION_LOG_DIR" >/dev/null
rg -l $'^verify-canic-backup\t1\t' "$CANIC_PUBLICATION_LOG_DIR" >/dev/null
rg -l $'^verify-canic-backup\t0\t' "$CANIC_PUBLICATION_LOG_DIR" >/dev/null
if rg -F 'unexpected Cargo command' "$FIXTURE/logs"; then exit 1; fi

for package in "${expected_packages[@]}"; do
    rm "$PUBLICATION_TEST_REGISTRY/$package"
done
: >"$PUBLICATION_TEST_EVENTS"
PUBLISH_DRY_RUN=1 run_case 0
for package in "${expected_packages[@]}"; do
    [[ ! -e "$PUBLICATION_TEST_REGISTRY/$package" ]]
done
[[ "$(rg -c '^publish .* --dry-run$' "$PUBLICATION_TEST_EVENTS")" -eq "${#expected_packages[@]}" ]]
rg -F 'publish -p canic-core --locked --no-verify --dry-run' "$PUBLICATION_TEST_EVENTS" >/dev/null
echo "publication runner fixtures passed (no registry effects)"
