#!/usr/bin/env bash
set -euo pipefail
ROOT="${BASH_SOURCE[0]}"
[[ "$ROOT" == /* ]] || ROOT="$PWD/$ROOT"
ROOT="$(CDPATH='' cd -P "${ROOT%/*}/../.." && printf '%s/.' "$PWD")"
ROOT="${ROOT%/.}"
# shellcheck source=scripts/ci/fixture-lifecycle.sh
source "$ROOT/scripts/ci/fixture-lifecycle.sh"
fixture="$(fixture_create "${TMPDIR:-/tmp}/canic-release-stage.XXXXXX")"
fixture_complete=false
trap 'fixture_finish "$?" "$fixture" "Release stage fixture retained"' EXIT
mkdir -p "$fixture/ci" "$fixture/make" "$fixture/scripts/ci" "$fixture/scripts/release"
cp "$ROOT/Makefile" "$ROOT/tool-versions.env" "$fixture/"
cp "$ROOT/ci/tool-versions.env" "$ROOT/ci/ic-tools.tsv" "$fixture/ci/"
cp "$ROOT/make/"*.mk "$fixture/make/"
cp "$ROOT/scripts/ci/check-make-execution.sh" "$ROOT/scripts/ci/ic-tool-pins.sh" "$fixture/scripts/ci/"
cp "$ROOT/scripts/release/adapter.sh" "$fixture/scripts/release/"
printf '#!/usr/bin/env bash\nprintf "0.112.0\\n"\n' > "$fixture/scripts/ci/read-workspace-version.sh"
cd "$fixture"
# Git only owns this disposable index. No commit, tag, push or release runs.
git init --quiet
for member in 'crates/space name/Cargo.toml' testing/probe/Cargo.toml; do
    mkdir -p "${member%/*}"
    printf 'member\n' > "$member"
    git add -- "$member"
done
RELEASE_VERSION=0.112.0 bash scripts/release/adapter.sh files > inventory.nul
while IFS= read -r -d '' path; do
    [[ "$path" != */* ]] || mkdir -p "${path%/*}"
    printf 'selected release bytes\n' > "$path"
done < inventory.nul
printf 'unrelated work\n' > unrelated.txt
make --no-print-directory -j4 release-stage > stage.log 2>&1
git ls-files -z > observed.nul
perl -0ne 'push @paths, $_; END { print sort @paths }' inventory.nul > expected.nul
cmp expected.nul observed.nul
while IFS= read -r -d '' path; do
    git show ":$path" > staged-bytes
    cmp "$path" staged-bytes
done < inventory.nul
# A partial failed inventory must never reach Git staging.
git ls-files --stage > index.before
printf 'later edit\n' >> Cargo.toml
cat > scripts/release/adapter.sh <<'STUB'
#!/usr/bin/env bash
printf 'Cargo.toml\0'
exit 42
STUB
status=0
make --no-print-directory release-stage > failed-stage.log 2>&1 || status=$?
[[ "$status" != 0 ]]
git ls-files --stage > index.after
cmp index.before index.after
fixture_complete=true
echo 'Release staging uses the complete NUL inventory and preserves the index on inventory failure'
