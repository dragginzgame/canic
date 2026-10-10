#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FIXTURE="$(mktemp -d "${TMPDIR:-/tmp}/canic-publish-runner.XXXXXX")"
finish() {
    local status=$?
    if [[ "$status" == 0 ]]; then rm -rf "$FIXTURE";
    else echo "Publication fixtures retained: $FIXTURE" >&2; fi
}
trap finish EXIT
mkdir -p "$FIXTURE/scripts/ci" "$FIXTURE/bin" "$FIXTURE/registry"
cp "$ROOT/scripts/ci/publish-workspace.sh" "$FIXTURE/scripts/ci/"
cp "$ROOT/scripts/ci/check-crates-io-version.sh" "$FIXTURE/scripts/ci/"
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
    publish)
        [[ "$2" == -p && "$4" == --locked ]] || exit 98
        [[ " $* " != *' --no-verify '* ]] || exit 97
        [[ "${FAKE_FAIL_PACKAGE:-}" != "$3" ]] || exit 37
        if [[ " $* " != *' --dry-run '* ]]; then
            touch "$PUBLICATION_TEST_REGISTRY/$3"
            if [[ "${FAKE_PROPAGATION_PACKAGE:-}" == "$3" ]]; then
                touch "$PUBLICATION_TEST_REGISTRY/$3-pending"
            fi
        fi
        ;;
    *) echo "unexpected Cargo command: $*" >&2; exit 99 ;;
esac
SH
cat >"$FIXTURE/bin/curl" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
url="${!#}"
package="${url%/0.1.0}"
package="${package##*/}"
[[ "$url" == "https://crates.io/api/v1/crates/$package/0.1.0" ]] || exit 99
printf 'lookup %s\n' "$package" >>"$PUBLICATION_TEST_EVENTS"
http=404
if [[ -f "$PUBLICATION_TEST_REGISTRY/$package" ]]; then http=200; fi
if [[ -f "$PUBLICATION_TEST_REGISTRY/$package-pending" ]]; then
    http="${FAKE_PROPAGATION_HTTP:-503}"
    if [[ -f "$PUBLICATION_TEST_REGISTRY/first-poll-absent" ]]; then
        rm "$PUBLICATION_TEST_REGISTRY/first-poll-absent"
        http=404
    fi
elif [[ "${FAKE_LOOKUP_PACKAGE:-}" == "$package" ]]; then
    http="${FAKE_LOOKUP_HTTP:-503}"
fi
printf '%s' "$http"
exit "${FAKE_REGISTRY_TRANSPORT:-0}"
SH
chmod +x "$FIXTURE/bin/cargo" "$FIXTURE/bin/curl"
export PUBLICATION_TEST_EVENTS="$FIXTURE/events"
export PUBLICATION_TEST_REGISTRY="$FIXTURE/registry"
export CANIC_PUBLICATION_LOG_DIR="$FIXTURE/logs"
export PUBLISH_FROM='' PUBLISH_DRY_RUN=0
expected_packages=(canic-backup canic-contracts canic-core canic-control-plane canic-macros canic canic-blob-service canic-host canic-cli)

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
CARGO_NET_OFFLINE=true run_case 2
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
lookup_calls="$(rg -c '^lookup ' "$PUBLICATION_TEST_EVENTS")"
run_case 0
[[ "$(rg -c '^publish ' "$PUBLICATION_TEST_EVENTS")" == "$publish_calls" ]]
[[ "$(( $(rg -c '^lookup ' "$PUBLICATION_TEST_EVENTS") - lookup_calls ))" -eq "${#expected_packages[@]}" ]]

# A resumed suffix must still reject a missing predecessor package.
rm "$PUBLICATION_TEST_REGISTRY/canic-backup"
PUBLISH_FROM=canic-host run_case 1
[[ "$(rg -c '^publish ' "$PUBLICATION_TEST_EVENTS")" == "$publish_calls" ]]
touch "$PUBLICATION_TEST_REGISTRY/canic-backup"
PUBLISH_FROM=canic-host run_case 0
FAKE_LOOKUP_PACKAGE=canic-backup FAKE_LOOKUP_HTTP=503 PUBLISH_FROM=canic-host run_case 2
[[ "$(rg -c '^publish ' "$PUBLICATION_TEST_EVENTS")" == "$publish_calls" ]]

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

# Unavailable initial observation never grants publication authority.
for http in 401 429 503 malformed; do
    : >"$PUBLICATION_TEST_EVENTS"
    FAKE_LOOKUP_PACKAGE=canic-backup FAKE_LOOKUP_HTTP="$http" run_case 2
    [[ "$(cat "$PUBLICATION_TEST_EVENTS")" == 'lookup canic-backup' ]]
done
for http in 200 404; do
    : >"$PUBLICATION_TEST_EVENTS"
    FAKE_LOOKUP_PACKAGE=canic-backup FAKE_LOOKUP_HTTP="$http" FAKE_REGISTRY_TRANSPORT=28 run_case 2
    [[ "$(cat "$PUBLICATION_TEST_EVENTS")" == 'lookup canic-backup' ]]
done

# A lost registry observation after upload stops the next package. One definite
# absence may be polled again, but a subsequent unknown observation cannot.
for initial_absence in 0 1; do
    : >"$PUBLICATION_TEST_EVENTS"
    if [[ "$initial_absence" == 1 ]]; then
        touch "$PUBLICATION_TEST_REGISTRY/first-poll-absent"
    fi
    FAKE_PROPAGATION_PACKAGE=canic-backup FAKE_PROPAGATION_HTTP=503 run_case 2
    [[ "$(rg -c '^publish ' "$PUBLICATION_TEST_EVENTS")" == 1 ]]
    rg -q '^publish -p canic-backup --locked$' "$PUBLICATION_TEST_EVENTS"
    [[ "$(rg -c '^lookup canic-backup$' "$PUBLICATION_TEST_EVENTS")" -eq "$((initial_absence + 2))" ]]
    rm "$PUBLICATION_TEST_REGISTRY/canic-backup" "$PUBLICATION_TEST_REGISTRY/canic-backup-pending"
done
rg -l $'^lookup-canic-backup\t2\t' "$CANIC_PUBLICATION_LOG_DIR" >/dev/null
rg -l $'^propagation-canic-backup\t2\t' "$CANIC_PUBLICATION_LOG_DIR" >/dev/null
rg -l $'^verify-canic-backup\t2\t' "$CANIC_PUBLICATION_LOG_DIR" >/dev/null
echo "publication runner fixtures passed (no registry effects)"
