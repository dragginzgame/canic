#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd -P)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/cloc-tooling-test.XXXXXX")"
trap 'if [[ $? == 0 ]]; then rm -rf "$fixture"; else printf "Failed tooling LOC fixture retained: %s\n" "$fixture" >&2; fi' EXIT
parent="$fixture/parent projects"
repo="$parent/a consumer"
mkdir -p "$parent"
# Reuse existing commit objects; no fixture commits or consumer commands run.
git clone -q --shared --no-checkout "$ROOT" "$repo"
# A legal repository name must not collide with the report's temporary files.
git clone -q --shared --no-checkout "$ROOT" "$parent/files"
mkdir -p "$repo/scripts/ci" "$repo/scripts/dev" "$repo/.github/workflows" "$repo/src" \
    "$repo/crates/example" "$repo/scripts/target" "$repo/scripts/__pycache__"
printf '# comment\nprintf "hello\\n"\n' > "$repo/scripts/ci/a.sh"
cp "$repo/scripts/ci/a.sh" "$repo/scripts/ci/b.sh"
printf '# jq comment\ntrue\n' > "$repo/scripts/ci/check.jq"
printf 'name: fixture\non: push\n' > "$repo/.github/workflows/test.yml"
printf '# comment\nprint "hello\\n";\n' > "$repo/scripts/dev/local.pl"
printf 'report:\n\t@exit 99\n' > "$repo/Makefile"
printf '[package]\nname = "example"\n' > "$repo/crates/example/Cargo.toml"
printf 'fn main() {}\n' > "$repo/crates/example/build.rs"
printf 'fn runtime_builder() {}\n' > "$repo/src/build.rs"
printf 'print "ignored";\n' > "$repo/scripts/target/generated.pl"
cp "$repo/scripts/target/generated.pl" "$repo/scripts/__pycache__/generated.pl"
printf 'scripts/ignored.sh\n' > "$repo/.gitignore"
cp "$repo/scripts/ci/a.sh" "$repo/scripts/ignored.sh"
printf '{"baseline":[1,2,3]}\n' > "$repo/scripts/ci/baseline.json"
printf '+added\n-removed\n' > "$repo/scripts/ci/input.patch"
printf 'name\tversion\n' > "$repo/scripts/ci/pins.tsv"
ln -s "$repo/scripts/dev/local.pl" "$repo/scripts/dev/alias.pl"
ln -s "$repo" "$parent/alias"
hash="$(bash "$ROOT/scripts/ci/verify-file-checksum.sh" --print sha256 "$repo/scripts/ci/a.sh")"
revision="$(git -C "$ROOT" rev-parse HEAD)"
printf 'format\t1\nsource\thttps://github.com/dragginzgame/shared-tooling.git\nrevision\t%s\nfile\t%s\t-\tscripts/ci/a.sh\n' \
    "$revision" "$hash" > "$repo/.shared-tooling.snapshot"
perl "$ROOT/scripts/dev/cloc-tooling.pl" --json "$parent" > "$fixture/counts.json"
jq -e '
  .partial == false and (.repositories | length) == 2 and
  .totals == {ci_loc:5,other_loc:4,total_loc:9,shared_loc:1,local_loc:8,data_lines:4} and
  .repositories[0].dirty == true and
  (.repositories[0].skipped_files | length) == 0 and
  (.repositories[0].files | map(.path) | index("src/build.rs")) == null and
  .repositories[1].totals.total_loc == 0
' "$fixture/counts.json" >/dev/null
perl "$ROOT/scripts/dev/cloc-tooling.pl" "$parent" > "$fixture/counts.txt"
[[ "$(awk 'END { print $1,$2,$3,$4,$5,$6,$7 }' "$fixture/counts.txt")" == 'TOTAL 5 4 9 1 8 4' ]]
# Custom manifest placement retains checkout-relative records for every source URL
# admitted by the canonical verifier. It must not relocate records to config/.
mkdir "$repo/config"
mv "$repo/.shared-tooling.snapshot" "$repo/config/.shared-tooling.snapshot"
for source in https://github.com/dragginzgame/shared-tooling.git git@github.com:dragginzgame/shared-tooling.git ssh://git@github.com/dragginzgame/shared-tooling; do
    sed "s|https://github.com/dragginzgame/shared-tooling.git|$source|" "$repo/config/.shared-tooling.snapshot" > "$fixture/manifest"
    cp "$fixture/manifest" "$repo/config/.shared-tooling-extra.snapshot"
    perl "$ROOT/scripts/dev/cloc-tooling.pl" --json "$parent" > "$fixture/custom.json"
    jq -e '.totals.shared_loc == 1 and .totals.local_loc == 8' "$fixture/custom.json" >/dev/null
done
rm "$repo/config/.shared-tooling-extra.snapshot"
mv "$repo/config/.shared-tooling.snapshot" "$repo/.shared-tooling.snapshot"
# Multiple manifests and nested snapshots retain their real shared ownership.
sed 's|scripts/ci/a.sh|scripts/ci/b.sh|' "$repo/.shared-tooling.snapshot" > "$repo/.shared-tooling-extra.snapshot"
mkdir -p "$repo/vendor/shared/scripts/ci"
cp "$repo/scripts/ci/a.sh" "$repo/vendor/shared/scripts/ci/c.sh"
cp "$repo/scripts/ci/a.sh" "$repo/vendor/shared/scripts/ci/d.sh"
sed 's|scripts/ci/a.sh|scripts/ci/c.sh|' "$repo/.shared-tooling.snapshot" > "$repo/vendor/shared/.shared-tooling.snapshot"
# A bundle with no canonical verifier needs an explicit root selection.
perl "$ROOT/scripts/dev/cloc-tooling.pl" --json --snapshot-root \
    "$repo/vendor/shared/.shared-tooling.snapshot" "$repo/vendor/shared" "$parent" > "$fixture/explicit.json"
jq -e '.totals.shared_loc == 3 and .totals.local_loc == 8' "$fixture/explicit.json" >/dev/null
# A normal bundle carries the verifier at its canonical path. This comment-only
# marker models that placement without adding source LOC or running the verifier.
printf '# verifier placement fixture\n' > "$repo/vendor/shared/scripts/ci/verify-shared-tooling-snapshot.sh"
perl "$ROOT/scripts/dev/cloc-tooling.pl" --json "$parent" > "$fixture/nested.json"
jq -e '.totals.total_loc == 11 and .totals.shared_loc == 3 and
  .totals.local_loc == 8 and (.repositories[0].snapshot_manifests | length) == 3' "$fixture/nested.json" >/dev/null
# Byte-identical but unrecorded copies stay local; modified snapshot files too.
printf 'printf "changed\\n"\n' >> "$repo/scripts/ci/a.sh"
perl "$ROOT/scripts/dev/cloc-tooling.pl" --json "$parent" > "$fixture/drift.json" 2> "$fixture/drift.err"
jq -e '.totals.shared_loc == 2 and .totals.local_loc == 10 and
  .repositories[0].drifted_files == ["scripts/ci/a.sh"]' "$fixture/drift.json" >/dev/null
[[ -s "$fixture/drift.err" ]]
# A mode-only difference must not be labelled an intact snapshot copy.
cp "$repo/scripts/ci/b.sh" "$repo/scripts/ci/a.sh"
chmod +x "$repo/scripts/ci/a.sh"
perl "$ROOT/scripts/dev/cloc-tooling.pl" --json "$parent" > "$fixture/mode.json" 2> "$fixture/mode.err"
jq -e '.totals.shared_loc == 2 and .totals.total_loc == 11' "$fixture/mode.json" >/dev/null
printf 'bad manifest\n' > "$repo/.shared-tooling.snapshot"
if TMPDIR="$fixture" perl "$ROOT/scripts/dev/cloc-tooling.pl" --json "$parent" \
    > "$fixture/failed.json" 2> "$fixture/failed.err"; then
    echo 'malformed snapshot was accepted' >&2; exit 1
fi
jq -e '.partial == true and .repositories[0].error != null and
  .repositories[1].totals.total_loc == 0' "$fixture/failed.json" >/dev/null
grep -F 'Failed tooling inventory retained:' "$fixture/failed.err" >/dev/null
if perl "$ROOT/scripts/dev/cloc-tooling.pl" "$fixture/absent" > /dev/null 2>&1; then exit 1; fi
if perl "$ROOT/scripts/dev/cloc-tooling.pl" --snapshot-root "$fixture/manifest" "$fixture" "$parent" > /dev/null 2>&1; then exit 1; fi
perl "$ROOT/scripts/dev/cloc-tooling.pl" --help >/dev/null
echo 'Tooling LOC, source/data separation and snapshot ownership tests passed'
