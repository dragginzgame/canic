#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=scripts/ci/fixture-lifecycle.sh
source "$ROOT/scripts/ci/fixture-lifecycle.sh"
scratch_parent="${TMPDIR:-$ROOT/target/tmp}"
mkdir -p "$scratch_parent"
fixture="$(fixture_create "$scratch_parent/canic-adoption-test.XXXXXX")"
fixture_complete=false
trap 'fixture_finish "$?" "$fixture" "Adoption fixtures retained"' EXIT
mkdir -p "$fixture/scripts/"{ci,dev} "$fixture/bin"
cp "$ROOT/scripts/dev/check-dependency-adoption.sh" "$fixture/scripts/dev/"
cp "$ROOT/scripts/dev/prepare-integration-workspace.pl" "$fixture/scripts/dev/"
git init -q "$fixture"
printf 'target/\n' > "$fixture/.gitignore"
cat > "$fixture/scripts/ci/check-cargo-dependency-boundaries.sh" <<'BOUNDARY'
#!/usr/bin/env bash
printf 'boundary\n' >> "$TRACE"
[[ "${FAIL_STAGE:-}" != boundary ]]
BOUNDARY
cat > "$fixture/bin/cargo" <<'CARGO'
#!/usr/bin/env bash
printf '%s\n' "$*" >> "$TRACE"
if [[ "${FAIL_STAGE:-}" == "$1" ]]; then exit 17; fi
case "$1" in
    tree)
        echo 'ic-memory v0.62.1'
        echo 'ic-memory v0.62.1 (*)'
        echo 'ic-platform v0.16.0'
        echo 'ic-platform v0.17.0'
        [[ "${MISSING:-}" == ic-platform-runtime ]] || echo 'ic-platform-runtime v0.17.1'
        [[ "${MISSING:-}" == ic-platform-runtime-guard ]] || echo 'ic-platform-runtime-guard v0.17.1'
        echo 'ic-storage v0.62.1'
        case "${DUPLICATE:-}" in
            yes) echo 'ic-memory v0.61.0' ;;
            ic-platform-runtime) echo 'ic-platform-runtime v0.16.0' ;;
            ic-platform-runtime-guard) echo 'ic-platform-runtime-guard v0.16.0' ;;
        esac
        ;;
esac
exit 0
CARGO
chmod +x "$fixture/bin/cargo"
export PATH="$fixture/bin:$PATH" TRACE="$fixture/trace"
script="$fixture/scripts/dev/check-dependency-adoption.sh"
: > "$TRACE"
if bash "$script" > "$fixture/no-args.log" 2>&1; then exit 1; fi
[[ ! -s "$TRACE" ]]
if bash "$script" --workspace > "$fixture/option.log" 2>&1; then exit 1; fi
[[ ! -s "$TRACE" ]]
bash "$script" canic-host canic-contracts > "$fixture/pass.log"
# All cheap observations precede Clippy; verifier is last. Repeated identities
# from a diamond dependency do not reject. Explicit selected packages survive.
awk '
 /^clippy / { lint=1; if ($0 !~ /--locked --offline/ || $0 !~ /--all-targets --all-features -- -D warnings/) exit 1 }
 /^(metadata |tree |boundary)/ { if (lint) exit 1 }
 /^run / { if (!lint || $0 !~ /--example verify_embedded_root$/) exit 1; verify=1 }
 END { if (!verify) exit 1 }
' "$TRACE"
grep -q -- '--package canic-host --package canic-contracts' "$TRACE"
for manifest in Cargo.toml integrations/blob-service/consumer/Cargo.toml integrations/blob-service/embedded-consumer/Cargo.toml; do
    grep '^metadata ' "$TRACE" | grep -Fq -- "--manifest-path $manifest"
done
for manifest in integrations/blob-service/consumer/Cargo.toml integrations/blob-service/embedded-consumer/Cargo.toml; do
    grep '^clippy ' "$TRACE" | grep -Fq -- "--manifest-path $fixture/target/integrations/workspace/$manifest"
done
for stage in metadata boundary tree clippy run; do
    : > "$TRACE"
    if FAIL_STAGE="$stage" bash "$script" canic-host > "$fixture/$stage.log" 2>&1; then exit 1; fi
    if [[ "$stage" != run ]]; then
        if grep -q '^run ' "$TRACE"; then exit 1; fi
    fi
    if [[ "$stage" != clippy && "$stage" != run ]]; then
        if grep -q '^clippy ' "$TRACE"; then exit 1; fi
    fi
done
: > "$TRACE"
if DUPLICATE=yes bash "$script" canic-host > "$fixture/duplicate.log" 2>&1; then exit 1; fi
grep -q 'ic-memory v0.61.0' "$fixture/duplicate.log"
if grep -q '^clippy ' "$TRACE"; then exit 1; fi
for owner in ic-platform-runtime ic-platform-runtime-guard; do
    for condition in duplicate missing; do
        : > "$TRACE"
        if [[ "$condition" == duplicate ]]; then
            if DUPLICATE="$owner" bash "$script" canic-host > "$fixture/$owner-$condition.log" 2>&1; then exit 1; fi
        else
            if MISSING="$owner" bash "$script" canic-host > "$fixture/$owner-$condition.log" 2>&1; then exit 1; fi
        fi
        grep -q "Conflicting $owner identities" "$fixture/$owner-$condition.log"
        if grep -q '^clippy ' "$TRACE"; then exit 1; fi
    done
done
: > "$TRACE"
bash "$script" --admission-only > "$fixture/admission.log"
if grep -Eq '^(clippy|run) ' "$TRACE"; then exit 1; fi
: > "$TRACE"
bash "$script" --fixtures-only > "$fixture/fixtures.log"
if grep -Eq '^(metadata|tree|boundary) ' "$TRACE"; then exit 1; fi
if grep '^clippy ' "$TRACE" | grep -q -- '--package'; then exit 1; fi
grep -q '^run ' "$TRACE"
if bash "$script" --fixtures-only canic-host > "$fixture/extra.log" 2>&1; then exit 1; fi
fixture_complete=true
echo 'Dependency adoption command fixtures passed.'
