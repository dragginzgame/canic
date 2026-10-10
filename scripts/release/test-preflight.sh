#!/usr/bin/env bash
set -euo pipefail

# Private Make invocations must not inherit the caller's release arguments or injected files.
unset MAKEFLAGS MFLAGS MAKEOVERRIDES GNUMAKEFLAGS MAKEFILES

# Exercise the actual Make/adapter boundary with private Cargo-cache and Git substitutes.
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-release-preflight.XXXXXX")"
trap 'if [[ $? == 0 ]]; then rm -rf -- "$fixture"; else printf "Preflight fixture retained: %s\n" "$fixture" >&2; fi' EXIT
mkdir -p "$fixture/bin" "$fixture/scripts/release" "$fixture/scripts/ci" "$fixture/ci" "$fixture/make"
cp "$root/Makefile" "$root/tool-versions.env" "$fixture/"
cp "$root/ci/tool-versions.env" "$root/ci/ic-tools.tsv" "$fixture/ci/"
cp "$root/make/tools.mk" "$root/make/execution.mk" "$root/make/release.mk" "$fixture/make/"
cp "$root/scripts/ci/check-make-execution.sh" "$fixture/scripts/ci/"
cp "$root/scripts/ci/ic-tool-pins.sh" "$fixture/scripts/ci/"
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
    'set-version --help') exit "${PREFLIGHT_TOOL_STATUS:-0}" ;;
    'fetch --locked')
        [[ "${PREFLIGHT_FETCH_STATUS:-0}" == 0 ]] || exit "$PREFLIGHT_FETCH_STATUS"
        if [[ "${CARGO_NET_OFFLINE:-false}" == true && ! -f "$PREFLIGHT_CACHE" ]]; then exit 71; fi
        touch "$PREFLIGHT_CACHE"
        ;;
    *) echo 'unexpected Cargo command or selection-changing flags' >&2; exit 99 ;;
esac
SH
cat > "$fixture/bin/make" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
case "$*" in
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
chmod +x "$fixture/bin/git" "$fixture/bin/cargo" "$fixture/bin/make"
real_make="$(command -v make)"
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
printf 'draft patch\ncargo fetch --locked offline=false\nsetup\ntools-check\ncargo set-version --help offline=false\n' > expected
cmp expected "$PREFLIGHT_EVENTS"
CARGO_NET_OFFLINE=true run_preflight success
rm -f "$PREFLIGHT_CACHE"
CARGO_NET_OFFLINE=true run_preflight failure
[[ ! -e "$PREFLIGHT_CACHE" ]]
PREFLIGHT_FETCH_STATUS=37 CARGO_NET_OFFLINE=false run_preflight failure
[[ ! -e "$PREFLIGHT_CACHE" ]]

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
PREFLIGHT_TOOL_STATUS=29 CARGO_NET_OFFLINE=false run_preflight failure
[[ "$(tail -1 "$PREFLIGHT_EVENTS")" == 'cargo set-version --help offline=false' ]]
PREFLIGHT_SETUP_STATUS=31 run_preflight failure
[[ "$(tail -1 "$PREFLIGHT_EVENTS")" == setup ]]
PREFLIGHT_CHECK_STATUS=32 run_preflight failure
[[ "$(tail -1 "$PREFLIGHT_EVENTS")" == tools-check ]]

# Make's actual validation recipe stays offline; the validation command is substituted.
: > "$PREFLIGHT_EVENTS"
PATH="$fixture/bin:$PATH" "$real_make" --no-print-directory \
    MAKE="$fixture/bin/make" release-verify > "$fixture/output" 2>&1
[[ "$(cat "$PREFLIGHT_EVENTS")" == 'validation offline=true' ]]
echo 'Release preflight cache preparation, offline policy, source admission and failure boundaries passed'
