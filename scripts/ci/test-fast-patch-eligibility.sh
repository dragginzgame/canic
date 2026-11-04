#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-fast-eligibility.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/bin" "$fixture/scripts/ci"
mkdir -p "$fixture/scripts/release" "$fixture/ci"
cp "$ROOT/scripts/release/selection.sh" "$fixture/scripts/release/"
cp "$ROOT/ci/cargo-release-groups.json" "$fixture/ci/"
export RELEASE_GROUP=framework
cp "$ROOT/scripts/ci/check-fast-patch-eligibility.sh" "$ROOT/scripts/ci/read-release-validation.sh" "$ROOT/scripts/ci/require-jq.sh" "$fixture/scripts/ci/"
printf '#!/usr/bin/env bash\necho 1.2.3\n' >"$fixture/scripts/ci/read-workspace-version.sh"
cat >"$fixture/bin/git" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[[ "$1" != -C ]] || shift 2
case "$*" in
    'status --porcelain' | 'merge-base --is-ancestor '* | 'cat-file -e '* | 'diff --check '*) ;;
    'cat-file -t refs/tags/framework-v1.2.3') echo tag ;;
    'rev-list -n 1 framework-v1.2.3') echo release ;;
    'rev-parse framework-v1.2.3:ci/cargo-release-groups.json') echo 2222222222222222222222222222222222222222 ;;
    'show framework-v1.2.3:releases/framework/release-validation.json')
        if [[ -n "${FAKE_RECEIPT+x}" ]]; then printf '%s\n' "$FAKE_RECEIPT"; else
            echo '{"schema":1,"group":"framework","policy":"2222222222222222222222222222222222222222","version":"1.2.3","source":"1111111111111111111111111111111111111111","date":"2026-10-01","gate":"complete"}'
        fi ;;
    'diff --name-only framework-v1.2.3..HEAD') printf '%s\n' "$CHANGED_PATHS" ;;
    *) echo "unexpected Git command: $*" >&2; exit 99 ;;
esac
SH
chmod +x "$fixture/bin/git"
export PATH="$fixture/bin:$PATH"
export CHANGED_PATHS=docs/note.md
bash "$fixture/scripts/ci/check-fast-patch-eligibility.sh" --eligibility-only >"$fixture/output" 2>&1
for path in Cargo.lock Cargo.toml crates/canic/src/lib.rs canisters/test/sharding_root_stub/src/lib.rs; do
    CHANGED_PATHS="$path"
    if bash "$fixture/scripts/ci/check-fast-patch-eligibility.sh" --eligibility-only >"$fixture/output" 2>&1; then
        echo "fast lane admitted behavior input: $path" >&2
        exit 1
    fi
done
cat >"$fixture/bin/make" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[[ "$*" == '--no-print-directory clippy' ]] || exit 99
printf 'clippy\n' >>"$FAST_GATE_EVENTS"
exit "${FAST_CLIPPY_STATUS:-0}"
SH
chmod +x "$fixture/bin/make"
for gate in check-current-document-semantics check-release-validation-matrix; do
    cat >"$fixture/scripts/ci/$gate.sh" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$(basename "$0" .sh)" >>"$FAST_GATE_EVENTS"
SH
done
export CHANGED_PATHS=docs/note.md FAST_GATE_EVENTS="$fixture/events"
status=0
FAST_CLIPPY_STATUS=23 bash "$fixture/scripts/ci/check-fast-patch-eligibility.sh" \
    >"$fixture/output" 2>&1 || status=$?
[[ "$status" == 23 && "$(cat "$FAST_GATE_EVENTS")" == clippy ]]
: >"$FAST_GATE_EVENTS"
bash "$fixture/scripts/ci/check-fast-patch-eligibility.sh" >"$fixture/output" 2>&1
printf '%s\n' clippy check-current-document-semantics check-release-validation-matrix \
    >"$fixture/expected"
cmp "$fixture/expected" "$FAST_GATE_EVENTS"
echo 'fast eligibility fixtures passed (docs admitted; behavior inputs require complete validation)'
for invalid in 'not json' '{}' \
    '{"schema":2,"group":"framework","version":"1.2.3"}' \
    '{"schema":1,"group":"icydb","policy":"2222222222222222222222222222222222222222","version":"1.2.3","source":"1111111111111111111111111111111111111111","date":"2026-10-01","gate":"complete"}' \
    '{"schema":1,"group":"framework","policy":"3333333333333333333333333333333333333333","version":"1.2.3","source":"1111111111111111111111111111111111111111","date":"2026-10-01","gate":"complete"}'; do
    if FAKE_RECEIPT="$invalid" bash "$fixture/scripts/ci/read-release-validation.sh" framework-v1.2.3 1.2.3 > "$fixture/output" 2>&1; then
        echo 'Invalid group receipt was accepted' >&2; exit 1
    fi
done
