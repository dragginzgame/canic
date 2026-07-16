#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CI="$ROOT/.github/workflows/ci.yml"
MAKEFILE="$ROOT/Makefile"
TOOLS="$ROOT/tool-versions.env"
MATRIX="$ROOT/docs/governance/supported-platforms.md"
VERIFY="$ROOT/scripts/ci/verify-file-checksum.sh"
ICP_REQUIRE="$ROOT/scripts/ci/require_icp.sh"
SECRET_SCAN="$ROOT/scripts/ci/run-secret-scan.sh"
GITLEAKS_IGNORE="$ROOT/.gitleaksignore"
DEPENDENCY_RISK_GATE="$ROOT/scripts/ci/check-dependency-risk-inventory.sh"
DEPENDENCY_RISK_INVENTORY="$ROOT/scripts/ci/dependency-risk-inventory.tsv"
BUMP_VERSION="$ROOT/scripts/ci/bump-version.sh"
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

for file in "$CI" "$MAKEFILE" "$TOOLS" "$MATRIX" "$VERIFY" "$ICP_REQUIRE" "$SECRET_SCAN" "$GITLEAKS_IGNORE" "$DEPENDENCY_RISK_GATE" "$DEPENDENCY_RISK_INVENTORY" "$BUMP_VERSION"; do
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
rg --multiline 'test-bump:[^\n]*\\\n[[:space:]]+gitleaks-scan' "$MAKEFILE" >/dev/null ||
    fail "the patch-release gate does not require the dedicated secret scan"
rg --multiline 'test-bump:[^\n]*\\\n[[:space:]]+gitleaks-scan dependency-risk-gate' "$MAKEFILE" >/dev/null ||
    fail "the patch-release gate does not require dependency risk validation"
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
sha512_count=0
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
        sha512_count=$((sha512_count + 1))
        ;;
    esac
done <"$TOOLS"

[ "$sha256_count" -gt 0 ] || fail "no SHA-256 pins were found"
[ "$sha512_count" -gt 0 ] || fail "no SHA-512 pins were found"

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

bash -n "$VERIFY" "${installers[@]}" "$SECRET_SCAN" "$ROOT/scripts/dev/install_dev.sh"

echo "release integrity contract guard passed ($external_action_count immutable Actions)"
