#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TOOLS="$ROOT/tool-versions.env"
CODEOWNERS="$ROOT/.github/CODEOWNERS"

fail() {
    echo "release integrity guard failed: $1" >&2
    exit 1
}

# Only authority records belong here. Command arguments, sequencing, cleanup,
# source immutability and failed-effect handling have executable owner tests.
for file in "$TOOLS" "$CODEOWNERS" "$ROOT/rust-toolchain.toml" "$ROOT/.github/workflows/ci.yml"; do
    [[ -f "$file" ]] || fail "missing required authority record: $file"
done

# Native ci_authority tests parse YAML and own Action pin, permission,
# credential and job-dependency validation. Workflow lint owns YAML syntax.
# CODEOWNERS entries are records: whitespace and additional owners are permitted.
for owned_path in \
    /.github/workflows/ \
    /.github/dependabot.yml \
    /.githooks/ \
    /Makefile \
    /scripts/ci/ \
    /rust-toolchain.toml \
    /tool-versions.env \
    /docs/governance/ci-deployment.md \
    /docs/governance/supported-platforms.md; do
    awk -v path="$owned_path" '
        $1 == path { for (i = 2; i <= NF; i++) if ($i == "@dragginzgame") found = 1 }
        END { exit !found }
    ' "$CODEOWNERS" || fail "CODEOWNERS is missing CI authority $owned_path"
done

# shellcheck source=/dev/null
source "$TOOLS"
mapfile -t pin_vars < <(env -i PATH="$PATH" bash -c 'source "$1"; compgen -A variable CANIC_' _ "$TOOLS")
version_count=0
sha256_count=0
for variable in "${pin_vars[@]}"; do
    value="${!variable:-}"
    case "$variable" in
        CANIC_*_SHA256*)
            [[ "$value" =~ ^[0-9a-f]{64}$ ]] || fail "invalid SHA-256 pin: $variable"
            sha256_count=$((sha256_count + 1))
            continue
            ;;
        CANIC_*_SHA512*)
            [[ "$value" =~ ^[0-9a-f]{128}$ ]] || fail "invalid SHA-512 pin: $variable"
            continue
            ;;
        CANIC_*_VERSION) version_count=$((version_count + 1)) ;;
        *) continue ;;
    esac
    if [[ "$variable" == CANIC_BINARYEN_VERSION ]]; then
        [[ "$value" =~ ^[1-9][0-9]*$ ]] || fail "$variable is not an exact Binaryen release"
    else
        [[ "$value" =~ ^[0-9]+\.[0-9]+\.[0-9]+([+-][0-9A-Za-z.-]+)?$ ]] ||
            fail "$variable is not an exact semantic version"
    fi
done
[[ "$version_count" -gt 0 ]] || fail "no exact tool-version pins were found"
[[ "$sha256_count" -gt 0 ]] || fail "no SHA-256 pins were found"

bash "$ROOT/scripts/ci/check-pocketic-version-alignment.sh"
echo "release integrity authority records passed"
