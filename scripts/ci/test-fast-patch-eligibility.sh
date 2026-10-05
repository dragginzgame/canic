#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-fast-eligibility.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/bin" "$fixture/scripts/ci"
cp "$ROOT/scripts/ci/check-fast-patch-eligibility.sh" "$ROOT/scripts/ci/read-release-validation.sh" "$ROOT/scripts/ci/require-jq.sh" "$fixture/scripts/ci/"
printf '#!/usr/bin/env bash\necho 1.2.3\n' >"$fixture/scripts/ci/read-workspace-version.sh"
cat >"$fixture/bin/git" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[[ "$1" != -C ]] || shift 2
case "$*" in
    'status --porcelain' | 'merge-base --is-ancestor '* | 'cat-file -e '* | 'diff --check '*) ;;
    'cat-file -t refs/tags/v1.2.3') echo tag ;;
    'rev-list -n 1 v1.2.3') echo release ;;
    'show v1.2.3:release-validation.json')
        echo '{"schema":1,"version":"1.2.3","source":"1111111111111111111111111111111111111111","date":"2026-10-01","gate":"complete"}' ;;
    'diff --name-only v1.2.3..HEAD') printf '%s\n' "$CHANGED_PATHS" ;;
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
echo 'fast eligibility fixtures passed (docs admitted; behavior inputs require complete validation)'
