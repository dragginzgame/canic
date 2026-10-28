#!/usr/bin/env bash
set -euo pipefail

# Private Make invocations must not inherit the caller's release arguments or injected files.
unset MAKEFLAGS MFLAGS MAKEOVERRIDES GNUMAKEFLAGS MAKEFILES
unset RELEASE_SOURCE

# Exercise the actual Make/adapter boundary with private Cargo-cache and Git substitutes.
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-release-preflight.XXXXXX")"
trap 'if [[ $? == 0 ]]; then rm -rf -- "$fixture"; else printf "Preflight fixture retained: %s\n" "$fixture" >&2; fi' EXIT
mkdir -p "$fixture/bin" "$fixture/scripts/release" "$fixture/scripts/ci" "$fixture/ci" "$fixture/make"
cp "$root/Makefile" "$root/tool-versions.env" "$fixture/"
cp "$root/ci/tool-versions.env" "$root/ci/ic-tools.tsv" "$fixture/ci/"
cp "$root/make/tools.mk" "$root/make/execution.mk" "$root/make/release.mk" "$root/make/consumer.mk" "$root/make/rust-format.mk" "$fixture/make/"
cp "$root/scripts/ci/check-make-execution.sh" "$fixture/scripts/ci/"
cp "$root/scripts/ci/ic-tool-pins.sh" "$fixture/scripts/ci/"
cp "$root/scripts/ci/"{run-release-validation-lane,prepare-validation}.sh "$fixture/scripts/ci/"
for tool in install-pinned-ci-tool testkit-server; do
    printf '#!/usr/bin/env bash\nprintf "tool %%s %%s\\n" "%s" "$*" >> "$PREFLIGHT_EVENTS"\n' "$tool" > "$fixture/scripts/ci/$tool.sh"
done
cp "$root/scripts/release/adapter.sh" "$fixture/scripts/release/"
printf '[workspace.package]\nversion = "1.2.3"\n' > "$fixture/Cargo.toml"
printf 'version = 4\n' > "$fixture/Cargo.lock"
cp "$fixture/Cargo.toml" "$fixture/manifest-original"
cp "$fixture/Cargo.lock" "$fixture/lock-original"
cat > "$fixture/scripts/ci/read-workspace-version.sh" <<'SH'
#!/usr/bin/env bash
printf '1.2.3\n'
SH
cat > "$fixture/scripts/ci/check-release-draft-ready.sh" <<'SH'
#!/usr/bin/env bash
printf 'draft %s\n' "$*" >> "$PREFLIGHT_EVENTS"
exit "${PREFLIGHT_DRAFT_STATUS:-0}"
SH
cat > "$fixture/bin/git" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
case "$*" in
    'rev-parse HEAD') echo 1111111111111111111111111111111111111111; exit 0 ;;
    'diff --name-only -z --') kind=unstaged ;;
    'diff --cached --name-only -z --') kind=staged ;;
    'ls-files --others --exclude-standard -z') kind=untracked ;;
    *) echo 'unexpected Git effect' >&2; exit 99 ;;
esac
if [[ "${PREFLIGHT_DIRTY_KIND:-}" == "$kind" ]]; then
    printf '%s\0' "${PREFLIGHT_DIRTY_PATH:-crates/member/src/lib.rs}"
fi
SH
cat > "$fixture/bin/cargo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf 'cargo %s offline=%s\n' "$*" "${CARGO_NET_OFFLINE:-unset}" >> "$PREFLIGHT_EVENTS"
case "$*" in
    'fetch --locked --manifest-path '* )
        [[ "${PREFLIGHT_FETCH_STATUS:-0}" == 0 ]] || exit "$PREFLIGHT_FETCH_STATUS"
        [[ "$4" != "${PREFLIGHT_FAIL_MANIFEST:-}" ]] || exit 38
        if [[ "${CARGO_NET_OFFLINE:-false}" == true && ! -f "$PREFLIGHT_CACHE" ]]; then exit 71; fi
        touch "$PREFLIGHT_CACHE"
        ;;
    'install cargo-audit --version '*|'install cargo-edit --version '*) ;;
    *) echo 'unexpected Cargo command or selection-changing flags' >&2; exit 99 ;;
esac
SH
cat > "$fixture/bin/make" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
case "$*" in
    '--no-print-directory prepare-validation SCOPE=release')
        exec bash scripts/ci/prepare-validation.sh release ;;
    '--no-print-directory prepare-build-environment'|'--no-print-directory check-build-environment')
        printf '%s\n' "$2" >> "$PREFLIGHT_EVENTS"
        exit "${PREFLIGHT_ENV_STATUS:-0}" ;;
    '--no-print-directory prepare-locked-dependencies')
        exec "$PREFLIGHT_REAL_MAKE" --no-print-directory prepare-locked-dependencies ;;
    '--no-print-directory install-host-tools'|'--no-print-directory install-rust-tools'|'--no-print-directory install-release-metadata-editor'|'--no-print-directory release-metadata-editor-check')
        printf 'make %s\n' "$*" >> "$PREFLIGHT_EVENTS"
        [[ "$2" != "${PREFLIGHT_FAIL_TOOL:-}" ]] || exit 39 ;;
    '--no-print-directory ensure-clean') exit 0 ;;
    '--no-print-directory install-tools')
        printf 'setup\n' >> "$PREFLIGHT_EVENTS"
        exit "${PREFLIGHT_SETUP_STATUS:-0}" ;;
    '--no-print-directory tools-check')
        printf 'tools-check\n' >> "$PREFLIGHT_EVENTS"
        exit "${PREFLIGHT_CHECK_STATUS:-0}" ;;
    '--no-print-directory validate')
        [[ "${CARGO_NET_OFFLINE:-}" == true ]]
        printf 'validation offline=%s\n' "$CARGO_NET_OFFLINE" >> "$PREFLIGHT_EVENTS" ;;
    *) exit 99 ;;
esac
SH
cat > "$fixture/bin/rustup" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[[ "$*" == 'target add wasm32-unknown-unknown' ]]
printf 'rustup %s\n' "$*" >> "$PREFLIGHT_EVENTS"
exit "${PREFLIGHT_TARGET_STATUS:-0}"
SH
chmod +x "$fixture/bin/git" "$fixture/bin/cargo" "$fixture/bin/make" "$fixture/bin/rustup"
real_make="$(command -v make)"
export PREFLIGHT_REAL_MAKE="$real_make"
export PREFLIGHT_EVENTS="$fixture/events" PREFLIGHT_CACHE="$fixture/cache-entry"
export RELEASE_PREVIOUS=1.2.3 RELEASE_KIND=patch RELEASE_VERSION=1.2.4
cd "$fixture"

run_preflight() {
    local expected="$1" status=0
    : > "$PREFLIGHT_EVENTS"
    PATH="$fixture/bin:$PATH" "$real_make" --no-print-directory release-preflight \
        > "$fixture/output" 2>&1 || status=$?
    if [[ "$expected" == success ]]; then
        [[ "$status" == 0 ]] || { cat "$fixture/output" >&2; exit 1; }
    else
        [[ "$status" != 0 ]] || { echo 'preflight accepted rejected inputs' >&2; exit 1; }
    fi
    cmp manifest-original Cargo.toml
    cmp lock-original Cargo.lock
}

# A cold cache is prepared without overriding an explicitly selected offline policy.
CARGO_NET_OFFLINE=false run_preflight success
[[ -f "$PREFLIGHT_CACHE" ]]
{
    echo 'draft patch'
    echo 'prepare-build-environment'
    for manifest in Cargo.toml integrations/blob-service/consumer/Cargo.toml integrations/blob-service/embedded-consumer/Cargo.toml; do
        echo "cargo fetch --locked --manifest-path $manifest offline=false"
    done
    printf 'rustup target add wasm32-unknown-unknown\nsetup\ntools-check\n'
    # Read the same immutable tool catalog used by preparation.
    source "$root/tool-versions.env"
    echo "cargo install cargo-audit --version $CANIC_CARGO_AUDIT_VERSION --locked offline=false"
    echo "cargo install cargo-edit --version $CANIC_CARGO_EDIT_VERSION --locked offline=false"
    printf 'tool install-pinned-ci-tool actionlint\ntool install-pinned-ci-tool shellcheck\n'
} > expected
cmp expected "$PREFLIGHT_EVENTS"
CARGO_NET_OFFLINE=true run_preflight success
rm -f "$PREFLIGHT_CACHE"
CARGO_NET_OFFLINE=true run_preflight failure
[[ ! -e "$PREFLIGHT_CACHE" ]]
PREFLIGHT_FETCH_STATUS=37 CARGO_NET_OFFLINE=false run_preflight failure
[[ ! -e "$PREFLIGHT_CACHE" ]]
for manifest in integrations/blob-service/consumer/Cargo.toml integrations/blob-service/embedded-consumer/Cargo.toml; do
    PREFLIGHT_FAIL_MANIFEST="$manifest" CARGO_NET_OFFLINE=false run_preflight failure
    [[ "$(tail -1 "$PREFLIGHT_EVENTS")" == "cargo fetch --locked --manifest-path $manifest offline=false" ]]
done
rm -f "$PREFLIGHT_CACHE"

# Source/draft refusals occur before cache preparation; tool checks follow setup.
for kind in unstaged staged untracked; do
    PREFLIGHT_DIRTY_KIND="$kind" run_preflight failure
    [[ ! -s "$PREFLIGHT_EVENTS" && ! -e "$PREFLIGHT_CACHE" ]]
done
for allowed in CHANGELOG.md docs/changelog/1.2.md; do
    PREFLIGHT_DIRTY_KIND=unstaged PREFLIGHT_DIRTY_PATH="$allowed" CARGO_NET_OFFLINE=false run_preflight success
done
PREFLIGHT_DRAFT_STATUS=23 run_preflight failure
[[ "$(cat "$PREFLIGHT_EVENTS")" == 'draft patch' ]]
PREFLIGHT_SETUP_STATUS=31 run_preflight failure
[[ "$(tail -1 "$PREFLIGHT_EVENTS")" == setup ]]
PREFLIGHT_TARGET_STATUS=33 run_preflight failure
[[ "$(tail -1 "$PREFLIGHT_EVENTS")" == 'rustup target add wasm32-unknown-unknown' ]]
PREFLIGHT_CHECK_STATUS=32 run_preflight failure
[[ "$(tail -1 "$PREFLIGHT_EVENTS")" == tools-check ]]

# CI scopes use the same preparation owner and locked graphs. No real tools run.
for scope in preflight security checks; do
    : > "$PREFLIGHT_EVENTS"
    PATH="$fixture/bin:$PATH" CARGO_NET_OFFLINE=true bash scripts/ci/prepare-validation.sh "$scope"
    [[ "$(grep -c '^cargo fetch --locked --manifest-path ' "$PREFLIGHT_EVENTS")" == 3 ]]
    case "$scope" in
        preflight)
            grep -Fx 'make --no-print-directory install-rust-tools' "$PREFLIGHT_EVENTS"
            grep -Fx 'make --no-print-directory install-release-metadata-editor' "$PREFLIGHT_EVENTS"
            grep -Fx 'make --no-print-directory release-metadata-editor-check' "$PREFLIGHT_EVENTS" ;;
        security) grep -F "cargo install cargo-audit --version $CANIC_CARGO_AUDIT_VERSION --locked" "$PREFLIGHT_EVENTS" ;;
        checks) grep -Fx 'make --no-print-directory install-rust-tools' "$PREFLIGHT_EVENTS" ;;
    esac
    if grep -Fxq setup "$PREFLIGHT_EVENTS"; then exit 1; fi
done
# Editor setup/admission failures stop before any later preparation or consumer.
for target in install-rust-tools install-release-metadata-editor release-metadata-editor-check; do
    : > "$PREFLIGHT_EVENTS"
    if PATH="$fixture/bin:$PATH" PREFLIGHT_FAIL_TOOL="$target" \
        bash scripts/ci/prepare-validation.sh preflight; then exit 1; fi
    [[ "$(tail -1 "$PREFLIGHT_EVENTS")" == "make --no-print-directory $target" ]]
done
: > "$PREFLIGHT_EVENTS"
if PATH="$fixture/bin:$PATH" bash scripts/ci/prepare-validation.sh unknown; then exit 1; fi
[[ ! -s "$PREFLIGHT_EVENTS" ]]

# Make's actual validation recipe stays offline; the validation command is substituted.
: > "$PREFLIGHT_EVENTS"
PATH="$fixture/bin:$PATH" "$real_make" --no-print-directory \
    MAKE="$fixture/bin/make" release-verify > "$fixture/output" 2>&1
printf 'check-build-environment\ntools-check\nvalidation offline=true\n' > "$fixture/expected"
cmp "$fixture/expected" "$PREFLIGHT_EVENTS"
echo 'Release preflight cache preparation, offline policy, source admission and failure boundaries passed'
