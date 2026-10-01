#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-release-commit.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/bin" "$fixture/scripts/ci"
cp "$ROOT/scripts/ci/commit-release.sh" "$fixture/scripts/ci/"
printf '#!/usr/bin/env bash\necho 1.2.3\n' >"$fixture/scripts/ci/read-workspace-version.sh"
cat >"$fixture/bin/git" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$EVENTS"
case "$*" in
    'rev-parse --verify refs/tags/v1.2.3') exit "${TAG_STATUS:-1}" ;;
    'rev-parse HEAD')
        if [[ -f "$CREATED" ]]; then echo release; else echo source; fi ;;
    'rev-parse HEAD^') echo "${PARENT:-source}" ;;
    'log -1 --format=%s HEAD') echo "${SUBJECT:-Release 1.2.3}" ;;
    'commit -m Release 1.2.3')
        [[ "${COMMIT_STATUS:-0}" == 0 ]] || exit "$COMMIT_STATUS"
        [[ "${NO_SUCCESSOR:-0}" == 1 ]] || touch "$CREATED" ;;
    'tag -a v1.2.3 release -m Release 1.2.3') exit "${TAG_CREATE_STATUS:-0}" ;;
    *) exit 99 ;;
esac
SH
chmod +x "$fixture/bin/git"
export EVENTS="$fixture/events" CREATED="$fixture/created"
export PATH="$fixture/bin:$PATH"
cd "$fixture"

run_case() {
    local expected="$1" status=0
    rm -f "$CREATED"
    : >"$EVENTS"
    bash scripts/ci/commit-release.sh >output.log 2>&1 || status=$?
    [[ "$status" == "$expected" ]] || { cat output.log >&2; exit 1; }
}
assert_no_tag() { ! rg -q '^tag ' "$EVENTS"; }

COMMIT_STATUS=23 run_case 23
assert_no_tag
NO_SUCCESSOR=1 run_case 1
assert_no_tag
PARENT=unexpected run_case 1
assert_no_tag
SUBJECT=unexpected run_case 1
assert_no_tag
TAG_STATUS=0 run_case 1
assert_no_tag
if rg -q '^commit ' "$EVENTS"; then exit 1; fi
run_case 0
[[ "$(tail -1 "$EVENTS")" == 'tag -a v1.2.3 release -m Release 1.2.3' ]]
TAG_CREATE_STATUS=31 run_case 31
echo 'release commit fixtures passed (fake Git only)'
