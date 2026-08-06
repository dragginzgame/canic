#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CI="$ROOT/.github/workflows/ci.yml"
MAKEFILE="$ROOT/Makefile"
TOOLS="$ROOT/tool-versions.env"
RUST_TOOLCHAIN="$ROOT/rust-toolchain.toml"
MATRIX="$ROOT/docs/governance/supported-platforms.md"
VERIFY="$ROOT/scripts/ci/verify-file-checksum.sh"
ICP_REQUIRE="$ROOT/scripts/ci/require_icp.sh"
ICP_MODEL="$ROOT/crates/canic-host/src/icp/model.rs"
ICP_PROOF="$ROOT/scripts/ci/blob-storage-cli-proof-lib.sh"
DEV_INSTALL="$ROOT/scripts/dev/install_dev.sh"
INSTALLING="$ROOT/INSTALLING.md"
README="$ROOT/README.md"
SECRET_SCAN="$ROOT/scripts/ci/run-secret-scan.sh"
GITLEAKS_IGNORE="$ROOT/.gitleaksignore"
DEPENDENCY_RISK_GATE="$ROOT/scripts/ci/check-dependency-risk-inventory.sh"
DEPENDENCY_RISK_TEST="$ROOT/scripts/ci/test-dependency-risk-inventory.sh"
DEPENDENCY_RISK_INVENTORY="$ROOT/scripts/ci/dependency-risk-inventory.tsv"
BUMP_VERSION="$ROOT/scripts/ci/bump-version.sh"
RELEASE_CLEANUP="$ROOT/scripts/ci/cleanup-release-artifacts.sh"
RELEASE_GATES="$ROOT/scripts/ci/run-release-gates.sh"
RELEASE_PUSH="$ROOT/scripts/ci/push-release.sh"
POCKET_IC_ALIGNMENT="$ROOT/scripts/ci/check-pocketic-version-alignment.sh"
WORKSPACE_TEST_INVENTORY="$ROOT/scripts/ci/workspace-test-inventory.tsv"
WORKSPACE_TEST_INVENTORY_GATE="$ROOT/scripts/ci/check-workspace-test-inventory.sh"
WORKSPACE_TEST_RUNNER="$ROOT/scripts/ci/run-workspace-tests.sh"
installers=(
    "$ROOT/scripts/ci/install-actionlint.sh"
    "$ROOT/scripts/ci/install-gitleaks.sh"
    "$ROOT/scripts/ci/install-shellcheck.sh"
    "$ROOT/scripts/ci/install-pocketic.sh"
    "$ROOT/scripts/ci/install-icp-cli.sh"
    "$ROOT/scripts/ci/install-ic-wasm.sh"
)

fail() {
    echo "release integrity guard failed: $1" >&2
    exit 1
}

for file in "$CI" "$MAKEFILE" "$TOOLS" "$RUST_TOOLCHAIN" "$MATRIX" "$VERIFY" "$ICP_REQUIRE" "$ICP_MODEL" "$ICP_PROOF" "$DEV_INSTALL" "$INSTALLING" "$README" "$SECRET_SCAN" "$GITLEAKS_IGNORE" "$DEPENDENCY_RISK_GATE" "$DEPENDENCY_RISK_TEST" "$DEPENDENCY_RISK_INVENTORY" "$BUMP_VERSION" "$RELEASE_CLEANUP" "$RELEASE_GATES" "$RELEASE_PUSH" "$POCKET_IC_ALIGNMENT" "$WORKSPACE_TEST_INVENTORY" "$WORKSPACE_TEST_INVENTORY_GATE" "$WORKSPACE_TEST_RUNNER"; do
    [ -f "$file" ] || fail "missing required file: $file"
done

external_action_count=0
while IFS= read -r uses_entry; do
    action="${uses_entry#uses:}"
    action="${action#"${action%%[![:space:]]*}"}"
    case "$action" in
    ./*) continue ;;
    esac
    external_action_count=$((external_action_count + 1))
    if [[ ! "$action" =~ @[0-9a-f]{40}$ ]]; then
        fail "external Action is not pinned to a full commit: $action"
    fi
done < <(rg -o --no-filename 'uses:[[:space:]]*[^[:space:]#]+' "$ROOT/.github/workflows" -g '*.yml' -g '*.yaml')

[ "$external_action_count" -gt 0 ] || fail "no external Actions were inspected"

runner_count="$(rg -c '^[[:space:]]+runs-on: ubuntu-24\.04$' "$CI")"
all_runner_count="$(rg -c '^[[:space:]]+runs-on:' "$CI")"
[ "$runner_count" -eq 4 ] && [ "$all_runner_count" -eq 4 ] ||
    fail "all four jobs must select the canonical ubuntu-24.04 host"
ic_wasm_install_count="$(rg -c 'bash scripts/ci/install-ic-wasm\.sh' "$CI")"
[ "$ic_wasm_install_count" -eq 3 ] ||
    fail "all three IC tool jobs must use the checksum-bound ic-wasm installer"
rg -F 'run: bash scripts/ci/check-release-integrity-contract.sh' "$CI" >/dev/null ||
    fail "release integrity guard is not active in CI"
rg -F 'BIN="$(bash scripts/ci/install-gitleaks.sh)"' "$CI" >/dev/null ||
    fail "CI does not use the checksum-bound Gitleaks installer"
rg -F 'run: bash scripts/ci/run-secret-scan.sh' "$CI" >/dev/null ||
    fail "the dedicated secret scan is not active in CI"
rg -F 'run: bash scripts/ci/check-dependency-risk-inventory.sh' "$CI" >/dev/null ||
    fail "the dependency risk inventory gate is not active in CI"
rg -F 'bash scripts/ci/test-dependency-risk-inventory.sh' "$CI" >/dev/null ||
    fail "the dependency risk rejection tests are not active in CI"
rg --multiline 'test-bump:[^\n]*\\\n[[:space:]]+gitleaks-scan' "$MAKEFILE" >/dev/null ||
    fail "the patch-release gate does not require the dedicated secret scan"
rg --multiline 'test-bump:[^\n]*\\\n[[:space:]]+gitleaks-scan dependency-risk-gate' "$MAKEFILE" >/dev/null ||
    fail "the patch-release gate does not require dependency risk validation"
rg --multiline 'test-bump:[^\n]*\\\n[[:space:]]+gitleaks-scan dependency-risk-gate \\\n[[:space:]]+control-plane-feature-gate clippy test$' "$MAKEFILE" >/dev/null ||
    fail "the patch/minor release gate does not require the complete workspace test target"
bash "$WORKSPACE_TEST_INVENTORY_GATE" >/dev/null ||
    fail "the workspace integration-test inventory is incomplete or invalid"
rg -F 'bash scripts/ci/check-workspace-test-inventory.sh' "$WORKSPACE_TEST_RUNNER" >/dev/null ||
    fail "the workspace test runner does not enforce its integration-test inventory"
rg -F 'run_serial_pocketic_test' "$WORKSPACE_TEST_RUNNER" >/dev/null ||
    fail "the workspace test runner does not isolate serial PocketIC execution"
CANIC_TEST_PLAN_ONLY=1 bash "$WORKSPACE_TEST_RUNNER" fast >/dev/null ||
    fail "the fast workspace test plan cannot be resolved"
CANIC_TEST_PLAN_ONLY=1 bash "$WORKSPACE_TEST_RUNNER" full >/dev/null ||
    fail "the full workspace test plan cannot be resolved"
for mode in patch minor major; do
    rg -F "bash scripts/ci/run-release-gates.sh $mode" "$MAKEFILE" >/dev/null ||
        fail "the $mode version target does not use release-gate cleanup"
done
rg --multiline 'release-push:\n\t@bash scripts/ci/check-release-push-ready\.sh\n\t@bash scripts/ci/cleanup-release-artifacts\.sh\n\t@CANIC_RELEASE_PUSH_READY=1 bash scripts/ci/push-release\.sh' "$MAKEFILE" >/dev/null ||
    fail "release push does not clean before its final atomic network update"
rg -F 'cargo clean' "$RELEASE_CLEANUP" >/dev/null ||
    fail "release cleanup does not clear Cargo build artifacts"
rg -F 'MAX_CARGO_CLEAN_ATTEMPTS=2' "$RELEASE_CLEANUP" >/dev/null ||
    fail "release cleanup does not bound its Cargo cleanup retry"
rg -F '.tmp/test-runtime' "$RELEASE_CLEANUP" >/dev/null ||
    fail "release cleanup does not clear repository-owned test scratch"
rg -F 'export TMPDIR="$ROOT/.tmp/test-runtime"' "$RELEASE_GATES" >/dev/null ||
    fail "release gates do not confine temporary files to repository-owned scratch"
rg -F 'git push --atomic origin' "$RELEASE_PUSH" >/dev/null ||
    fail "release push does not require one atomic remote update"
rg -F '"HEAD:refs/heads/$branch"' "$RELEASE_PUSH" >/dev/null ||
    fail "release push does not name the exact branch ref"
rg -F '"refs/tags/$tag:refs/tags/$tag"' "$RELEASE_PUSH" >/dev/null ||
    fail "release push does not name the exact release tag ref"
if bash "$RELEASE_PUSH" >/dev/null 2>&1; then
    fail "release push helper accepted direct unverified invocation"
fi
if rg -F 'rm -rf -- /tmp' "$RELEASE_CLEANUP" >/dev/null; then
    fail "release cleanup may not delete an unscoped global temporary path"
fi
rg -F -- '--redact=100' "$SECRET_SCAN" >/dev/null ||
    fail "the dedicated secret scan does not redact findings"
rg -F '"$GITLEAKS_BIN" git' "$SECRET_SCAN" >/dev/null ||
    fail "the dedicated scanner does not inspect Git history"
rg -F -- '--gitleaks-ignore-path "$ROOT_DIR/.gitleaksignore"' "$SECRET_SCAN" >/dev/null ||
    fail "the dedicated secret scan does not select the reviewed fingerprint file"
rg -F 'Gitleaks configuration overrides are not allowed' "$SECRET_SCAN" >/dev/null ||
    fail "the dedicated secret scan does not reject external rule configuration"
rg -F 'repository .gitleaks.toml overrides are not allowed' "$SECRET_SCAN" >/dev/null ||
    fail "the dedicated secret scan does not reject repository rule configuration"
rg -F -- '--is-shallow-repository' "$SECRET_SCAN" >/dev/null ||
    fail "the dedicated secret scan does not reject incomplete Git history"
rg -F '[ "$version_output" != "$CANIC_GITLEAKS_VERSION" ]' "$SECRET_SCAN" >/dev/null ||
    fail "the dedicated secret scan does not require the exact Gitleaks version"
rg -F '[ "$version_output" != "$VERSION" ]' "$ROOT/scripts/ci/install-gitleaks.sh" >/dev/null ||
    fail "the Gitleaks installer does not require the exact reported version"
rg -F 'cargo update --workspace --offline' "$BUMP_VERSION" >/dev/null ||
    fail "the release bump does not preserve locked external dependency identities"

gitleaks_ignore_count=0
while IFS= read -r fingerprint; do
    case "$fingerprint" in
    '' | \#*) continue ;;
    esac
    [[ "$fingerprint" =~ ^[0-9a-f]{40}:.+:[a-z0-9-]+:[0-9]+$ ]] ||
        fail "invalid Gitleaks fingerprint entry"
    gitleaks_ignore_count=$((gitleaks_ignore_count + 1))
done <"$GITLEAKS_IGNORE"
[ "$gitleaks_ignore_count" -gt 0 ] || fail "no reviewed Gitleaks fingerprints were found"

while IFS= read -r install_command; do
    if [[ "$install_command" != *"--version"* ]]; then
        fail "CI Cargo helper install lacks an exact version: $install_command"
    fi
done < <(rg '^[[:space:]]*cargo install ' "$CI")

# shellcheck source=/dev/null
source "$TOOLS"

rust_toolchain="$(
    sed -n 's/^channel = "\([^"]*\)"$/\1/p' "$RUST_TOOLCHAIN"
)"
[ -n "$rust_toolchain" ] || fail "rust-toolchain.toml does not declare a channel"
rg -F "CANIC_INTERNAL_TOOLCHAIN: $rust_toolchain" "$CI" >/dev/null ||
    fail "CI internal Rust does not match rust-toolchain.toml"
rg -F "CANIC_RUST_TOOLCHAIN=\"\${CANIC_RUST_TOOLCHAIN:-$rust_toolchain}\"" "$DEV_INSTALL" >/dev/null ||
    fail "developer bootstrap Rust does not match rust-toolchain.toml"
rg -F "internal%20rust-$rust_toolchain-orange.svg" "$README" >/dev/null ||
    fail "README internal Rust badge does not match rust-toolchain.toml"
rg -F 'pins internal Rust `'"$rust_toolchain"'`' "$README" >/dev/null ||
    fail "README internal Rust text does not match rust-toolchain.toml"

icp_cli_minor_floor="${CANIC_ICP_CLI_VERSION%.*}.0"
rg -F "REQUIRED_ICP_CLI_VERSION: &str = \"$icp_cli_minor_floor\"" "$ICP_MODEL" >/dev/null ||
    fail "host ICP CLI minimum does not match the pinned minor line"
rg -F "ICP_CLI_SUPPORTED_VERSION_RANGE: &str = \">=$icp_cli_minor_floor, <2.0.0\"" "$ICP_MODEL" >/dev/null ||
    fail "host ICP CLI range does not match the pinned minor line"
rg -F "echo \"icp-cli $CANIC_ICP_CLI_VERSION\"" "$ICP_PROOF" >/dev/null ||
    fail "CLI proof fixture does not report the pinned ICP CLI version"
rg -F 'maintainer toolchain currently pins `'"$CANIC_ICP_CLI_VERSION"'`' "$INSTALLING" >/dev/null ||
    fail "installation guidance does not report the pinned ICP CLI version"

mapfile -t version_vars < <(
    sed -n 's/^export \(CANIC_[A-Z0-9_]*_VERSION\)=.*/\1/p' "$TOOLS"
)
[ "${#version_vars[@]}" -gt 0 ] || fail "no exact tool-version pins were found"
declare -A validated_version_vars=()
for variable in "${version_vars[@]}"; do
    value="${!variable:-}"
    [[ "$value" =~ ^[0-9]+\.[0-9]+\.[0-9]+([+-][0-9A-Za-z.-]+)?$ ]] ||
        fail "$variable is not an exact semantic version"
    validated_version_vars["$variable"]=1
done

sha256_count=0
declare -A validated_checksum_vars=()
while IFS='=' read -r variable digest; do
    case "$variable" in
    export\ CANIC_*_SHA256*)
        [[ "$digest" =~ ^[0-9a-f]{64}$ ]] || fail "invalid SHA-256 pin: $variable"
        validated_checksum_vars["${variable#export }"]=1
        sha256_count=$((sha256_count + 1))
        ;;
    export\ CANIC_*_SHA512*)
        [[ "$digest" =~ ^[0-9a-f]{128}$ ]] || fail "invalid SHA-512 pin: $variable"
        validated_checksum_vars["${variable#export }"]=1
        ;;
    esac
done <"$TOOLS"

[ "$sha256_count" -gt 0 ] || fail "no SHA-256 pins were found"

for installer in "${installers[@]}"; do
    rg -F 'verify-file-checksum.sh' "$installer" >/dev/null ||
        fail "installer does not verify downloaded content: $installer"
    rg -F -- "--proto-redir '=https'" "$installer" >/dev/null ||
        fail "installer does not constrain redirect protocols: $installer"
    rg '\$CANIC_[A-Z0-9_]*_VERSION' "$installer" >/dev/null ||
        fail "installer does not use a repository version pin: $installer"
    rg '\$CANIC_[A-Z0-9_]+_SHA(256|512)_[A-Z0-9_]+' "$installer" >/dev/null ||
        fail "installer does not use a repository checksum pin: $installer"
done

mapfile -t referenced_version_vars < <(
    rg -o --no-filename '\$CANIC_[A-Z0-9_]*_VERSION' \
        "${installers[@]}" "$ICP_REQUIRE" | sed 's/^\$//' | sort -u
)
[ "${#referenced_version_vars[@]}" -gt 0 ] ||
    fail "tool consumers do not reference repository version pins"
for variable in "${referenced_version_vars[@]}"; do
    [ -n "${validated_version_vars[$variable]:-}" ] ||
        fail "tool consumer references an unvalidated version pin: $variable"
done

mapfile -t referenced_checksum_vars < <(
    rg -o --no-filename '\$CANIC_[A-Z0-9_]+_SHA(256|512)_[A-Z0-9_]+' \
        "${installers[@]}" | sed 's/^\$//' | sort -u
)
[ "${#referenced_checksum_vars[@]}" -gt 0 ] ||
    fail "installers do not reference repository checksum pins"
for variable in "${referenced_checksum_vars[@]}"; do
    [ -n "${validated_checksum_vars[$variable]:-}" ] ||
        fail "installer references an unvalidated checksum pin: $variable"
done

caller_override_result="$(
    CANIC_ICP_CLI_VERSION=0.0.0 CANIC_IC_WASM_VERSION=0.0.0 \
        bash -c 'source "$1"; printf "%s %s\n" "$CANIC_ICP_CLI_VERSION" "$CANIC_IC_WASM_VERSION"' \
        _ "$ICP_REQUIRE"
)"
[ "$caller_override_result" = "$CANIC_ICP_CLI_VERSION $CANIC_IC_WASM_VERSION" ] ||
    fail "caller values can override the canonical IC tool pins"

if wrong_ic_wasm_output="$(
    bash -c '
        source "$1"
        icp() { printf "icp-cli %s\n" "$CANIC_ICP_CLI_VERSION"; }
        ic-wasm() { printf "ic-wasm 0.0.0\n"; }
        require_icp_tools
    ' _ "$ICP_REQUIRE" 2>&1
)"; then
    fail "the IC prerequisite check accepted an unpinned ic-wasm version"
fi
[[ "$wrong_ic_wasm_output" == *"unsupported ic-wasm version for Canic CI"* ]] ||
    fail "the IC prerequisite check did not preserve its version-mismatch cause"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

release_cleanup_fixture="$tmp_dir/release-cleanup"
release_cleanup_bin="$release_cleanup_fixture/bin"
mkdir -p \
    "$release_cleanup_fixture/scripts/ci" \
    "$release_cleanup_fixture/.tmp/test-runtime" \
    "$release_cleanup_fixture/target" \
    "$release_cleanup_bin"
cp "$RELEASE_CLEANUP" "$RELEASE_GATES" "$release_cleanup_fixture/scripts/ci/"
# shellcheck disable=SC2016 # Preserve expansion for the generated fixture.
printf '%s\n' \
    '#!/usr/bin/env bash' \
    'printf "%s\n" "$*" >"$PWD/gate-targets"' \
    'printf "%s\n" "$TMPDIR" >"$PWD/gate-tmpdir"' \
    'exit "${FAKE_MAKE_STATUS:-0}"' >"$release_cleanup_bin/make"
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
chmod +x "$release_cleanup_bin/make" "$release_cleanup_bin/cargo"

PATH="$release_cleanup_bin:$PATH" \
    bash "$release_cleanup_fixture/scripts/ci/run-release-gates.sh" patch
[ "$(cat "$release_cleanup_fixture/gate-targets")" = "test-bump" ] ||
    fail "patch release-gate wrapper did not select the patch gate"
[ "$(cat "$release_cleanup_fixture/gate-tmpdir")" = "$release_cleanup_fixture/.tmp/test-runtime" ] ||
    fail "release-gate wrapper did not confine temporary files to repository scratch"
[ ! -e "$release_cleanup_fixture/target" ] ||
    fail "successful release-gate cleanup retained Cargo artifacts"
[ ! -e "$release_cleanup_fixture/.tmp/test-runtime" ] ||
    fail "successful release-gate cleanup retained test scratch"

mkdir -p \
    "$release_cleanup_fixture/.tmp/test-runtime" \
    "$release_cleanup_fixture/target"
rm -f "$release_cleanup_fixture/cargo-clean-attempts"
if FAKE_MAKE_STATUS=23 PATH="$release_cleanup_bin:$PATH" \
    bash "$release_cleanup_fixture/scripts/ci/run-release-gates.sh" major; then
    fail "release-gate wrapper accepted a failed validation gate"
else
    release_gate_status=$?
fi
[ "$release_gate_status" -eq 23 ] ||
    fail "release-gate wrapper did not preserve the validation failure"
[ "$(cat "$release_cleanup_fixture/gate-targets")" = "control-plane-feature-gate clippy test" ] ||
    fail "major release-gate wrapper did not select the full release gates"
[ -e "$release_cleanup_fixture/target" ] ||
    fail "failed release-gate cleanup removed Cargo artifacts needed for retry"
[ ! -e "$release_cleanup_fixture/cargo-clean-attempts" ] ||
    fail "failed release-gate cleanup invoked Cargo clean"
[ ! -e "$release_cleanup_fixture/.tmp/test-runtime" ] ||
    fail "failed release-gate cleanup retained test scratch"

rm -f "$release_cleanup_fixture/cargo-clean-attempts"
mkdir -p \
    "$release_cleanup_fixture/.tmp/test-runtime" \
    "$release_cleanup_fixture/target"
FAKE_CARGO_FAILURES=1 PATH="$release_cleanup_bin:$PATH" \
    bash "$release_cleanup_fixture/scripts/ci/run-release-gates.sh" minor
[ "$(cat "$release_cleanup_fixture/cargo-clean-attempts")" -eq 2 ] ||
    fail "release cleanup did not retry one transient Cargo failure exactly once"
[ ! -e "$release_cleanup_fixture/target" ] ||
    fail "retried release cleanup retained Cargo artifacts"
[ ! -e "$release_cleanup_fixture/.tmp/test-runtime" ] ||
    fail "retried release cleanup retained test scratch"

rm -f "$release_cleanup_fixture/cargo-clean-attempts"
mkdir -p \
    "$release_cleanup_fixture/.tmp/test-runtime" \
    "$release_cleanup_fixture/target"
if FAKE_CARGO_FAILURES=2 FAKE_CARGO_STATUS=19 PATH="$release_cleanup_bin:$PATH" \
    bash "$release_cleanup_fixture/scripts/ci/run-release-gates.sh" minor; then
    fail "release-gate wrapper accepted a failed cleanup"
else
    release_cleanup_status=$?
fi
[ "$release_cleanup_status" -eq 1 ] ||
    fail "release-gate wrapper did not preserve the cleanup failure"
[ "$(cat "$release_cleanup_fixture/cargo-clean-attempts")" -eq 2 ] ||
    fail "release cleanup exceeded its bounded Cargo retry"
[ -e "$release_cleanup_fixture/target" ] ||
    fail "failed fake Cargo cleanup unexpectedly removed its target fixture"
[ ! -e "$release_cleanup_fixture/.tmp/test-runtime" ] ||
    fail "Cargo cleanup failure prevented test-scratch cleanup"

release_push_fixture="$tmp_dir/release-push"
release_push_bin="$release_push_fixture/bin"
mkdir -p "$release_push_fixture/scripts/ci" "$release_push_bin"
cp "$RELEASE_PUSH" "$release_push_fixture/scripts/ci/"
printf '%s\n' \
    '[workspace.package]' \
    'version = "0.101.10"' >"$release_push_fixture/Cargo.toml"
# shellcheck disable=SC2016 # Preserve argument handling for the generated fixture.
printf '%s\n' \
    '#!/usr/bin/env bash' \
    'case "${1:-}" in' \
    'symbolic-ref) printf "main\n" ;;' \
    'push) printf "%s\n" "$@" >"$PWD/push-arguments" ;;' \
    '*) exit 2 ;;' \
    'esac' >"$release_push_bin/git"
chmod +x "$release_push_bin/git"
CANIC_RELEASE_PUSH_READY=1 PATH="$release_push_bin:$PATH" \
    bash "$release_push_fixture/scripts/ci/push-release.sh"
expected_push_arguments=$'push\n--atomic\norigin\nHEAD:refs/heads/main\nrefs/tags/v0.101.10:refs/tags/v0.101.10'
[ "$(cat "$release_push_fixture/push-arguments")" = "$expected_push_arguments" ] ||
    fail "release push did not send the exact branch and tag refs atomically"

fake_gitleaks="$tmp_dir/gitleaks"
# shellcheck disable=SC2016 # Preserve variable expansion for the generated fixture.
printf '%s\n' \
    '#!/usr/bin/env bash' \
    'case "${1:-}" in' \
    'version)' \
    '    [ "${FAKE_GITLEAKS_VERSION_FAIL:-0}" != "1" ] || exit 1' \
    '    printf "%s\\n" "${FAKE_GITLEAKS_VERSION:-}"' \
    '    ;;' \
    'git) exit 0 ;;' \
    '*) exit 2 ;;' \
    'esac' >"$fake_gitleaks"
chmod +x "$fake_gitleaks"

if unavailable_gitleaks_output="$(
    FAKE_GITLEAKS_VERSION_FAIL=1 GITLEAKS_BIN="$fake_gitleaks" bash "$SECRET_SCAN" 2>&1
)"; then
    fail "the secret scan accepted unavailable Gitleaks version output"
fi
[[ "$unavailable_gitleaks_output" == *"unable to read the gitleaks version"* ]] ||
    fail "the secret scan did not preserve its unavailable-version cause"

if near_gitleaks_output="$(
    FAKE_GITLEAKS_VERSION="${CANIC_GITLEAKS_VERSION}0" \
        GITLEAKS_BIN="$fake_gitleaks" bash "$SECRET_SCAN" 2>&1
)"; then
    fail "the secret scan accepted a near-match Gitleaks version"
fi
[[ "$near_gitleaks_output" == *"gitleaks version mismatch"* ]] ||
    fail "the secret scan did not preserve its version-mismatch cause"

for config_variable in GITLEAKS_CONFIG GITLEAKS_CONFIG_TOML; do
    if config_override_output="$(
        env "$config_variable=review-override" \
            FAKE_GITLEAKS_VERSION="$CANIC_GITLEAKS_VERSION" \
            GITLEAKS_BIN="$fake_gitleaks" bash "$SECRET_SCAN" 2>&1
    )"; then
        fail "the secret scan accepted $config_variable"
    fi
    [[ "$config_override_output" == *"configuration overrides are not allowed"* ]] ||
        fail "the secret scan did not preserve its configuration-override cause"
done

fake_bin="$tmp_dir/bin"
mkdir -p "$fake_bin"
# shellcheck disable=SC2016 # Preserve argument handling for the generated fixture.
printf '%s\n' \
    '#!/usr/bin/env bash' \
    'last=""' \
    'for argument in "$@"; do last="$argument"; done' \
    'case "$last" in' \
    '--is-inside-work-tree) exit 0 ;;' \
    '--is-shallow-repository) printf "true\\n" ;;' \
    '*) exit 2 ;;' \
    'esac' >"$fake_bin/git"
chmod +x "$fake_bin/git"
if shallow_history_output="$(
    PATH="$fake_bin:$PATH" \
        FAKE_GITLEAKS_VERSION="$CANIC_GITLEAKS_VERSION" \
        GITLEAKS_BIN="$fake_gitleaks" bash "$SECRET_SCAN" 2>&1
)"; then
    fail "the secret scan accepted incomplete Git history"
fi
[[ "$shallow_history_output" == *"complete repository history is unavailable in a shallow clone"* ]] ||
    fail "the secret scan did not preserve its shallow-history cause"

if rg -n 'curl[^|]*\|' "${installers[@]}" "$ROOT/scripts/dev/install_dev.sh" >/dev/null; then
    fail "active installer pipes an unverified download into execution"
fi

rg -F 'runs-on: ubuntu-24.04' "$CI" >/dev/null ||
    fail "CI does not select the canonical supported host"
rg -F 'Ubuntu 24.04, x86_64' "$MATRIX" >/dev/null ||
    fail "supported host matrix is missing the CI host"
rg -F '`x86_64-unknown-linux-gnu`' "$MATRIX" >/dev/null ||
    fail "supported host matrix is missing the native target"
rg -F '`wasm32-unknown-unknown`' "$MATRIX" >/dev/null ||
    fail "supported host matrix is missing the canister target"
rg -F 'Install-Capable But Not Release-Supported' "$MATRIX" >/dev/null ||
    fail "supported host matrix does not distinguish installer branches"

printf 'canic-release-integrity\n' >"$tmp_dir/input"
bash "$VERIFY" sha256 \
    ef57c7341ccbad50924ce5ffe7d2069b1106acac606f1f8ebd92b5b0a47067df \
    "$tmp_dir/input"
if bash "$VERIFY" sha256 \
    0000000000000000000000000000000000000000000000000000000000000000 \
    "$tmp_dir/input" >"$tmp_dir/rejection.stdout" 2>"$tmp_dir/rejection.stderr"; then
    fail "checksum mismatch was accepted"
fi
rg -F 'sha256 checksum mismatch' "$tmp_dir/rejection.stderr" >/dev/null ||
    fail "checksum mismatch did not preserve its deterministic cause"

bash -n "$VERIFY" "${installers[@]}" "$SECRET_SCAN" "$POCKET_IC_ALIGNMENT" "$ROOT/scripts/dev/install_dev.sh"
bash "$POCKET_IC_ALIGNMENT" >/dev/null

echo "release integrity contract guard passed ($external_action_count immutable Actions)"
