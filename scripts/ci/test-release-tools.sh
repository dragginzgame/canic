#!/usr/bin/env bash
set -euo pipefail

# Expected rejection diagnostics stay captured unless a fixture actually fails.
if [[ "${1:-}" != --run-fixtures ]]; then
    [[ "$#" -eq 0 ]] || exit 2
    fixture_log="$(mktemp)"
    trap 'rm -f "$fixture_log"' EXIT
    if bash "$0" --run-fixtures >"$fixture_log" 2>&1; then
        echo "release tool behavior passed (isolated external commands)"
    else
        fixture_status=$?
        cat "$fixture_log" >&2
        exit "$fixture_status"
    fi
    exit 0
fi
[[ "$#" -eq 1 ]] || exit 2

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VERIFY="$ROOT/scripts/ci/verify-file-checksum.sh"
ICP_REQUIRE="$ROOT/scripts/ci/require_icp.sh"
RELEASE_CLEANUP="$ROOT/scripts/ci/cleanup-release-artifacts.sh"
TEST_SCRATCH_RUNNER="$ROOT/scripts/ci/run-with-test-scratch.sh"
SCCACHE_WRAPPER="$ROOT/scripts/ci/run-canic-sccache.sh"
SCCACHE_LAUNCHER="$ROOT/scripts/ci/run-sccache.sh"
POCKET_IC_STOPPER="$ROOT/scripts/ci/stop-owned-pocketic-servers.sh"
RELEASE_PUSH="$ROOT/scripts/ci/push-release.sh"
VERSION_READER="$ROOT/scripts/ci/read-workspace-version.sh"
TAG_DELETE_TEST="$ROOT/scripts/ci/test-tag-maintenance.pl"
release_clean_recipe="$(sed -n '/^release-clean:/,/^$/p' "$ROOT/Makefile")"
# shellcheck source=/dev/null
source "$ROOT/tool-versions.env"
# shellcheck source=/dev/null
source "$ROOT/scripts/ci/ic-tool-pins.sh"

fail() {
    echo "release tool behavior failed: $1" >&2
    exit 1
}

caller_override_result="$(
    CANIC_ICP_CLI_VERSION=0.0.0 CANIC_IC_WASM_VERSION=0.0.0 \
        bash -c 'source "$1"; printf "%s %s\n" "$CANIC_ICP_CLI_VERSION" "$CANIC_IC_WASM_VERSION"' \
        _ "$ICP_REQUIRE"
)"
[ "$caller_override_result" = "$CANIC_ICP_CLI_VERSION $CANIC_IC_WASM_VERSION" ] ||
    fail "caller values can override the canonical IC tool pins"

if bash -c '
        source "$1"
        icp() { printf "icp-cli %s\n" "$CANIC_ICP_CLI_VERSION"; }
        ic-wasm() { printf "ic-wasm 0.0.0\n"; }
        require_icp_tools
    ' _ "$ICP_REQUIRE" >/dev/null 2>&1; then
    fail "the IC prerequisite check accepted an unpinned ic-wasm version"
fi

tmp_dir="$(mktemp -d)"
cleanup() {
    for fixture_pid in "${owned_server_pid:-}" "${foreign_server_pid:-}"; do
        [[ -n "$fixture_pid" ]] || continue
        kill -KILL "$fixture_pid" 2>/dev/null || :
        wait "$fixture_pid" 2>/dev/null || :
    done
    rm -rf "$tmp_dir"
}
trap cleanup EXIT

release_cleanup_fixture="$tmp_dir/release-cleanup"
release_cleanup_bin="$release_cleanup_fixture/bin"
foreign_test_scratch="$release_cleanup_fixture/.tmp/test-runtime.FOREIGN"
mkdir -p \
    "$release_cleanup_fixture/scripts/ci" \
    "$foreign_test_scratch" \
    "$release_cleanup_fixture/target" \
    "$release_cleanup_bin"
touch "$foreign_test_scratch/live-owner"
cp "$RELEASE_CLEANUP" "$TEST_SCRATCH_RUNNER" "$SCCACHE_LAUNCHER" "$SCCACHE_WRAPPER" "$POCKET_IC_STOPPER" \
    "$release_cleanup_fixture/scripts/ci/"
# shellcheck disable=SC2016 # Preserve expansion for the generated fixture.
printf '%s\n' \
    '#!/usr/bin/env bash' \
    '[ "${1:-}" = "clean" ] || exit 2' \
    'attempt_file="$PWD/cargo-clean-attempts"' \
    'attempt=0' \
    '[ ! -f "$attempt_file" ] || read -r attempt <"$attempt_file"' \
    'attempt=$((attempt + 1))' \
    'printf "%s\n" "$attempt" >"$attempt_file"' \
    '[ "$attempt" -gt "${FAKE_CARGO_FAILURES:-0}" ] || exit "${FAKE_CARGO_STATUS:-19}"' \
    'rm -rf -- "$PWD/target"' >"$release_cleanup_bin/cargo"
chmod +x "$release_cleanup_bin/cargo"

assert_owned_test_scratch_cleaned() {
    local scratch name

    scratch="$(cat "$release_cleanup_fixture/test-tmpdir")"
    name="${scratch##*/}"
    [ "${scratch%/*}" = "$release_cleanup_fixture/.tmp" ] &&
        [[ "$name" =~ ^test-runtime\.[[:alnum:]]{6}$ ]] ||
        fail "test runner did not select one private repository scratch directory"
    [ ! -e "$scratch" ] ||
        fail "test runner retained its invocation-owned test scratch"
    [ -f "$foreign_test_scratch/live-owner" ] ||
        fail "test cleanup deleted another invocation's test scratch"
}

PATH="$release_cleanup_bin:$PATH" \
    bash "$release_cleanup_fixture/scripts/ci/run-with-test-scratch.sh" \
    bash -c 'printf "%s\n" "$TMPDIR" >"$1"' _ "$release_cleanup_fixture/test-tmpdir"
assert_owned_test_scratch_cleaned

# shellcheck disable=SC2016 # Preserve expansion for the generated fixture.
printf '%s\n' \
    '#!/usr/bin/env bash' \
    'printf "%s\n" "$TMPDIR" >"$FAKE_SCCACHE_RECORD.tmpdir"' \
    'printf "%s\n" "$SCCACHE_SERVER_UDS" >"$FAKE_SCCACHE_RECORD.socket"' \
    'printf "%s\n" "$*" >"$FAKE_SCCACHE_RECORD.args"' \
    'touch "$TMPDIR/server-owned-temp"' >"$release_cleanup_bin/sccache"
chmod +x "$release_cleanup_bin/sccache"
FAKE_SCCACHE_RECORD="$release_cleanup_fixture/sccache-record" \
    CANIC_SCCACHE_BIN="$release_cleanup_bin/sccache" \
    RUSTC_WRAPPER="$release_cleanup_fixture/scripts/ci/run-canic-sccache.sh" \
    bash "$release_cleanup_fixture/scripts/ci/run-with-test-scratch.sh" \
    "$release_cleanup_fixture/scripts/ci/run-canic-sccache.sh" --show-stats
[ "$(cat "$release_cleanup_fixture/sccache-record.tmpdir")" = \
    "$release_cleanup_fixture/.tmp/sccache-runtime/tmp" ] ||
    fail "sccache inherited invocation-owned test scratch"
[ "$(cat "$release_cleanup_fixture/sccache-record.socket")" = \
    "$release_cleanup_fixture/.tmp/sccache-runtime/server.sock" ] ||
    fail "sccache did not use its repository-owned server socket"
[ -f "$release_cleanup_fixture/.tmp/sccache-runtime/tmp/server-owned-temp" ] ||
    fail "test cleanup deleted the persistent sccache runtime"

env -u RUSTC_WRAPPER PATH="$release_cleanup_bin:$PATH" \
    FAKE_SCCACHE_RECORD="$release_cleanup_fixture/sccache-record" \
    CANIC_SCCACHE_BIN="$release_cleanup_bin/sccache" \
    bash "$release_cleanup_fixture/scripts/ci/run-with-test-scratch.sh" \
    bash -c 'exec "$RUSTC_WRAPPER" --show-stats'
[ "$(cat "$release_cleanup_fixture/sccache-record.tmpdir")" = \
    "$release_cleanup_fixture/.tmp/sccache-runtime/tmp" ] ||
    fail "direct targeted runner did not select the persistent cache wrapper"
for selected_wrapper in "" /explicit/compiler-wrapper; do
    RUSTC_WRAPPER="$selected_wrapper" \
        bash "$release_cleanup_fixture/scripts/ci/run-with-test-scratch.sh" \
        bash -c '[[ -v RUSTC_WRAPPER && "$RUSTC_WRAPPER" == "$1" ]]' _ "$selected_wrapper" ||
        fail "targeted runner replaced an explicit compiler wrapper"
done

# Failure evidence remains discoverable after the owner stops its processes.
status=0
PATH="$release_cleanup_bin:$PATH" \
    bash "$release_cleanup_fixture/scripts/ci/run-with-test-scratch.sh" \
    bash -c 'printf "%s\n" "$TMPDIR" > "$1"; printf "partial diagnostics\n" > "$TMPDIR/raw.log"; exit 101' \
    _ "$release_cleanup_fixture/failed-tmpdir" > "$release_cleanup_fixture/failed.log" 2>&1 || status=$?
[[ "$status" -eq 101 ]] || fail "scratch owner changed the test failure status"
failed_scratch="$(cat "$release_cleanup_fixture/failed-tmpdir")"
[[ -f "$failed_scratch/raw.log" ]] || fail "scratch owner deleted partial failure evidence"
rg -Fq "$failed_scratch" "$release_cleanup_fixture/failed.log" || fail "retained evidence path was not printed"
CANIC_TEST_SCRATCH="$failed_scratch" \
    bash "$release_cleanup_fixture/scripts/ci/cleanup-release-artifacts.sh" --scratch-only
[[ ! -e "$failed_scratch" ]] || fail "explicit cleanup could not delete retained evidence"

rm -f "$release_cleanup_fixture/cargo-clean-attempts"
mkdir -p "$release_cleanup_fixture/target"
FAKE_CARGO_FAILURES=1 PATH="$release_cleanup_bin:$PATH" \
    bash "$release_cleanup_fixture/scripts/ci/cleanup-release-artifacts.sh"
[ "$(cat "$release_cleanup_fixture/cargo-clean-attempts")" -eq 2 ] ||
    fail "release cleanup did not retry one transient Cargo failure exactly once"
[ ! -e "$release_cleanup_fixture/target" ] ||
    fail "retried release cleanup retained Cargo artifacts"
[ -f "$foreign_test_scratch/live-owner" ] ||
    fail "explicit Cargo cleanup deleted another invocation's test scratch"

rm -f "$release_cleanup_fixture/cargo-clean-attempts"
mkdir -p "$release_cleanup_fixture/target"
if FAKE_CARGO_FAILURES=2 FAKE_CARGO_STATUS=19 PATH="$release_cleanup_bin:$PATH" \
    bash "$release_cleanup_fixture/scripts/ci/cleanup-release-artifacts.sh"; then
    fail "explicit cleanup accepted a failed Cargo cleanup"
else
    release_cleanup_status=$?
fi
[ "$release_cleanup_status" -eq 1 ] ||
    fail "explicit cleanup did not preserve the Cargo cleanup failure"
[ "$(cat "$release_cleanup_fixture/cargo-clean-attempts")" -eq 2 ] ||
    fail "release cleanup exceeded its bounded Cargo retry"
[ -e "$release_cleanup_fixture/target" ] ||
    fail "failed fake Cargo cleanup unexpectedly removed its target fixture"

printf '%s\n' "$release_clean_recipe" >"$release_cleanup_fixture/Makefile"
rm -f "$release_cleanup_fixture/cargo-clean-attempts"
post_release_cleanup_log="$release_cleanup_fixture/post-release-cleanup.log"
if ! FAKE_CARGO_FAILURES=2 PATH="$release_cleanup_bin:$PATH" \
    make --no-print-directory -s -C "$release_cleanup_fixture" release-clean \
    >"$post_release_cleanup_log" 2>&1; then
    fail "post-release cleanup changed a successful release into a failed command"
fi
[ -e "$release_cleanup_fixture/target" ] ||
    fail "failed post-release cleanup unexpectedly removed its target fixture"

borrowed_test_scratch="$release_cleanup_fixture/.tmp/test-runtime.BORROW"
mkdir -p "$borrowed_test_scratch"
CANIC_TEST_SCRATCH="$borrowed_test_scratch" \
    bash "$release_cleanup_fixture/scripts/ci/run-with-test-scratch.sh" \
    bash -c '[ "$TMPDIR" = "$CANIC_TEST_SCRATCH" ]'
[ -d "$borrowed_test_scratch" ] ||
    fail "nested test runner deleted scratch owned by its caller"
CANIC_TEST_SCRATCH="$borrowed_test_scratch" \
    bash "$release_cleanup_fixture/scripts/ci/cleanup-release-artifacts.sh" --scratch-only
[ ! -e "$borrowed_test_scratch" ] ||
    fail "explicit scratch owner could not clear its private directory"

touch "$release_cleanup_fixture/.tmp/path-escape-sentinel"
if CANIC_TEST_SCRATCH="$release_cleanup_fixture/.tmp/test-runtime.BAD123/.." \
    bash "$release_cleanup_fixture/scripts/ci/cleanup-release-artifacts.sh" --scratch-only; then
    fail "release cleanup accepted a non-direct scratch target"
fi
[ -f "$release_cleanup_fixture/.tmp/path-escape-sentinel" ] ||
    fail "release cleanup followed an unowned scratch path"

ln -s "$foreign_test_scratch" "$release_cleanup_fixture/.tmp/test-runtime.LINK12"
if CANIC_TEST_SCRATCH="$release_cleanup_fixture/.tmp/test-runtime.LINK12" \
    bash "$release_cleanup_fixture/scripts/ci/cleanup-release-artifacts.sh" --scratch-only; then
    fail "release cleanup accepted a symlinked scratch target"
fi
[ -f "$foreign_test_scratch/live-owner" ] ||
    fail "release cleanup followed a symlink into another invocation's scratch"

owned_server_scratch="$release_cleanup_fixture/.tmp/test-runtime.SERVER"
owned_server_port="$owned_server_scratch/pocket_ic_12345.port"
foreign_server_port="$foreign_test_scratch/pocket_ic_67890.port"
mkdir -p "$owned_server_scratch"
touch "$owned_server_port" "$foreign_server_port"
bash -c 'exec -a pocket-ic bash -c "while :; do sleep 1; done" -- --port-file "$1"' \
    _ "$owned_server_port" &
owned_server_pid=$!
bash -c 'exec -a pocket-ic bash -c "while :; do sleep 1; done" -- --port-file "$1"' \
    _ "$foreign_server_port" &
foreign_server_pid=$!
sleep 0.1
CANIC_TEST_SCRATCH="$owned_server_scratch" \
    bash "$release_cleanup_fixture/scripts/ci/cleanup-release-artifacts.sh" --scratch-only
wait "$owned_server_pid" 2>/dev/null || :
if kill -0 "$owned_server_pid" 2>/dev/null; then
    fail "release cleanup retained its invocation-owned PocketIC server"
fi
kill -0 "$foreign_server_pid" 2>/dev/null ||
    fail "release cleanup stopped another invocation's PocketIC server"
[ ! -e "$owned_server_scratch" ] ||
    fail "release cleanup retained scratch after its PocketIC server stopped"
kill -KILL "$foreign_server_pid" 2>/dev/null || :
wait "$foreign_server_pid" 2>/dev/null || :
unset owned_server_pid foreign_server_pid

release_push_fixture="$tmp_dir/release-push"
release_push_bin="$release_push_fixture/bin"
mkdir -p "$release_push_fixture/scripts/ci" "$release_push_bin"
cp "$RELEASE_PUSH" "$VERSION_READER" "$ROOT/scripts/ci/read-cargo-workspace-version.sh" "$release_push_fixture/scripts/ci/"
printf '%s\n' \
    '[workspace.package]' \
    'version = "9.9.9"' >"$release_push_fixture/Cargo.toml"
printf '%s\n' \
    '[workspace.package]' \
    'version = "0.101.10"' >"$release_push_fixture/committed-Cargo.toml"
mkdir "$release_push_fixture/committed"
cp "$release_push_fixture/committed-Cargo.toml" "$release_push_fixture/committed/Cargo.toml"
# shellcheck disable=SC2016 # Preserve argument handling for the generated fixture.
printf '%s\n' \
    '#!/usr/bin/env bash' \
    '[[ "${1:-}" != -C ]] || shift 2' \
    'case "${1:-}" in' \
    'symbolic-ref) printf "main\n" ;;' \
    'archive) tar -cf - -C "$PWD/committed" . ;;' \
    'push) printf "%s\n" "$@" >"$PWD/push-arguments" ;;' \
    '*) exit 2 ;;' \
    'esac' >"$release_push_bin/git"
chmod +x "$release_push_bin/git"
CANIC_RELEASE_PUSH_READY=1 PATH="$release_push_bin:$PATH" \
    bash "$release_push_fixture/scripts/ci/push-release.sh"
expected_push_arguments=$'push\n--no-follow-tags\n--atomic\norigin\nHEAD:refs/heads/main\nrefs/tags/v0.101.10:refs/tags/v0.101.10'
[ "$(cat "$release_push_fixture/push-arguments")" = "$expected_push_arguments" ] ||
    fail "release push did not send the exact branch and tag refs atomically"

perl "$TAG_DELETE_TEST" >/dev/null ||
    fail "historical-tag deletion fixture failed"

# Exercise the authority guard with equivalent record layout and real corruption.
authority_fixture="$tmp_dir/authority"
mkdir -p "$authority_fixture/scripts/ci" "$authority_fixture/.github/workflows"
cp "$ROOT/scripts/ci/check-release-integrity-contract.sh" \
    "$ROOT/scripts/ci/check-pocketic-version-alignment.sh" "$authority_fixture/scripts/ci/"
cp "$ROOT/Cargo.lock" "$ROOT/rust-toolchain.toml" "$authority_fixture/"
mkdir -p "$authority_fixture/ci"
cp "$ROOT/ci/ic-tools.tsv" "$authority_fixture/ci/"
cp "$ROOT/scripts/ci/ic-tool-pins.sh" "$authority_fixture/scripts/ci/"
cp "$ROOT/.github/workflows/ci.yml" "$authority_fixture/.github/workflows/"
awk '{gsub(/ /, "\t"); print}' "$ROOT/.github/CODEOWNERS" >"$authority_fixture/.github/CODEOWNERS"
sed 's/^\(export CANIC_[A-Z0-9_]*=\)\(.*\)$/\1"\2"/' \
    "$ROOT/tool-versions.env" >"$authority_fixture/tool-versions.env"
printf '# Equivalent ShellCheck exclusion order: --exclude=SC2016,SC2001\n' \
    >"$authority_fixture/Makefile"
env -u POCKET_IC_BIN bash "$authority_fixture/scripts/ci/check-release-integrity-contract.sh" >/dev/null ||
    fail "authority guard rejected equivalent whitespace, quoting or unrelated source"
cp "$authority_fixture/tool-versions.env" "$tmp_dir/authority-pins"
for corruption in \
    'export CANIC_CARGO_EDIT_VERSION=latest' \
    'export CANIC_BINARYEN_WASM_OPT_SHA256_LINUX_X64=invalid'; do
    cp "$tmp_dir/authority-pins" "$authority_fixture/tool-versions.env"
    printf '%s\n' "$corruption" >>"$authority_fixture/tool-versions.env"
    if env -u POCKET_IC_BIN bash "$authority_fixture/scripts/ci/check-release-integrity-contract.sh" >/dev/null 2>&1; then
        fail "authority guard accepted a corrupt tool identity"
    fi
done
cp "$tmp_dir/authority-pins" "$authority_fixture/tool-versions.env"
awk '$1 != "/scripts/ci/"' "$ROOT/.github/CODEOWNERS" >"$authority_fixture/.github/CODEOWNERS"
if env -u POCKET_IC_BIN bash "$authority_fixture/scripts/ci/check-release-integrity-contract.sh" >/dev/null 2>&1; then
    fail "authority guard accepted missing CI ownership"
fi

printf 'canic-release-integrity\n' >"$tmp_dir/input"
bash "$VERIFY" sha256 \
    ef57c7341ccbad50924ce5ffe7d2069b1106acac606f1f8ebd92b5b0a47067df \
    "$tmp_dir/input"
if bash "$VERIFY" sha256 \
    0000000000000000000000000000000000000000000000000000000000000000 \
    "$tmp_dir/input" >"$tmp_dir/rejection.stdout" 2>"$tmp_dir/rejection.stderr"; then
    fail "checksum mismatch was accepted"
fi
echo "release tool behavior passed (isolated external commands)"
