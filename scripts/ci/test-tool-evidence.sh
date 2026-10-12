#!/usr/bin/env bash
# Shared companions: scripts/ci/select-tool-evidence.sh scripts/ci/archive-evidence.sh .github/actions/retain-failure-evidence/action.yml
set -euo pipefail
[[ $# == 3 ]] || { echo 'usage: test-tool-evidence.sh CONSUMER host|ic CALLER-PINS' >&2; exit 2; }
ROOT="${BASH_SOURCE[0]}"
[[ "$ROOT" == /* ]] || ROOT="$PWD/$ROOT"
ROOT="$(cd -P "${ROOT%/*}/../.." && printf '%s/.' "$PWD")"
ROOT="${ROOT%/.}"
consumer="$1"; kind="$2"; pins="$3"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/tool-evidence-test.XXXXXX")"
# Bash 3.2 can enter EXIT with status zero after nounset; require completion too.
fixture_complete=false
finish() {
    local status=$?
    [[ "$fixture_complete" == true || "$status" != 0 ]] || status=1
    if [[ "$status" == 0 ]]; then rm -rf "$fixture"
    else printf "Tool evidence fixture retained: %s\n" "$fixture" >&2; fi
    exit "$status"
}
trap finish EXIT
active="$(readlink "$consumer/.tools/$kind")"
physical="$(cd "$consumer/.tools/$active" && pwd -P)"
stored_parent="${physical%/*}"
stored_path="${stored_parent##*/}/${physical##*/}"
candidate="$consumer/.tools/$kind-set.evidenceCandidate"
mkdir "$candidate"
printf 'failed candidate\n' > "$candidate/payload"
chmod 640 "$candidate/payload"
ln -s missing "$candidate/link"
choose_evidence() {
    local mode="$1" destination="$2"
    mkdir "$destination"
    if [[ "$mode" == full ]]; then
        bash "$ROOT/scripts/ci/select-tool-evidence.sh" full "$consumer" "$destination" > "$destination/selections.nul"
    else
        bash "$ROOT/scripts/ci/select-tool-evidence.sh" compact "$consumer" "$destination" "$pins" "$pins" > "$destination/selections.nul"
    fi
}
archive() {
    local directory="$1" output="$2" root path
    local inputs=()
    while IFS= read -r -d '' root && IFS= read -r -d '' path; do inputs+=("$root" "$path"); done < "$directory/selections.nul"
    inputs+=("$fixture" "${directory#"$fixture/"}")
    bash "$ROOT/scripts/ci/archive-evidence.sh" "$output" "${inputs[@]}" > /dev/null
}
choose_evidence full "$fixture/full"
archive "$fixture/full" "$fixture/full.tar.gz"
mkdir "$fixture/full-unpacked"
tar -xzf "$fixture/full.tar.gz" -C "$fixture/full-unpacked"
[[ -d "$fixture/full-unpacked/$stored_path" ]] || exit 1
choose_evidence compact "$fixture/compact"
archive "$fixture/compact" "$fixture/compact.tar.gz"
mkdir "$fixture/compact-unpacked"
tar -xzf "$fixture/compact.tar.gz" -C "$fixture/compact-unpacked"
[[ ! -e "$fixture/compact-unpacked/$stored_path" && -s "$fixture/compact/$kind/check.log" ]] || exit 1
cmp "$pins" "$fixture/compact-unpacked/compact/$kind/caller-pins"
cmp "$candidate/payload" "$fixture/compact-unpacked/.tools/$kind-set.evidenceCandidate/payload"
[[ -L "$fixture/compact-unpacked/.tools/$kind-set.evidenceCandidate/link" ]] || exit 1
[[ "$(perl -e 'printf "%o", (stat($ARGV[0]))[2]&0777' "$fixture/compact-unpacked/.tools/$kind-set.evidenceCandidate/payload")" == 640 ]] || exit 1
if [[ "$kind" == ic ]]; then
    for receipt in pins.tsv host files.sha256; do cmp "$consumer/.tools/$active/$receipt" "$fixture/compact/ic/$receipt"; done
fi
printf 'Synthetic %s archive bytes: full=%s compact=%s\n' "$kind" "$(wc -c < "$fixture/full.tar.gz")" "$(wc -c < "$fixture/compact.tar.gz")"

# A fresh failed check keeps the active set, despite its successful installation.
payload="$consumer/.tools/$active/bin/"; [[ "$kind" != host ]] || payload+=jq
[[ "$kind" != ic ]] || payload+=quill
cp -p "$payload" "$fixture/original"
printf '\nchanged\n' >> "$payload"
choose_evidence compact "$fixture/corrupt"
archive "$fixture/corrupt" "$fixture/corrupt.tar.gz"
mkdir "$fixture/corrupt-unpacked"
tar -xzf "$fixture/corrupt.tar.gz" -C "$fixture/corrupt-unpacked"
cmp "$payload" "$fixture/corrupt-unpacked/$stored_path/bin/${payload##*/}"
cp -p "$fixture/original" "$payload"

# Unmanaged, absent, dangling and newline-bearing links never suppress bundles.
unknown_index=0
for target in absent missing "$consumer/.tools/$active" "$active"$'\n'; do
    rm -f "$consumer/.tools/$kind"
    [[ "$target" == absent ]] || ln -s "$target" "$consumer/.tools/$kind"
    unknown_index=$((unknown_index + 1))
    selection="$fixture/unknown.$unknown_index"
    choose_evidence compact "$selection"
    archive "$selection" "$selection.tar.gz"
    mkdir "$selection-unpacked"
    tar -xzf "$selection.tar.gz" -C "$selection-unpacked"
    [[ -d "$selection-unpacked/$stored_path" ]] || exit 1
done
rm -f "$consumer/.tools/$kind"
ln -s "$active" "$consumer/.tools/$kind"

# A check of one selection cannot authorize suppressing a later selection.
mkdir "$fixture/race-bin" "$consumer/.tools/$kind-set.evidenceRace"
cat > "$fixture/race-bin/bash" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail
"$EVIDENCE_REAL_BASH" "$@"
case "${1:-}" in
    */install-"$EVIDENCE_RACE_KIND"-tools.sh)
        rm "$EVIDENCE_RACE_CONSUMER/.tools/$EVIDENCE_RACE_KIND"
        ln -s "$EVIDENCE_RACE_KIND-set.evidenceRace" "$EVIDENCE_RACE_CONSUMER/.tools/$EVIDENCE_RACE_KIND"
        ;;
esac
SCRIPT
# Use an absolute interpreter so resolving bash on PATH cannot recurse.
printf '#!%s\n' "$BASH" > "$fixture/race-bin/bash.tmp"
tail -n +2 "$fixture/race-bin/bash" >> "$fixture/race-bin/bash.tmp"
mv "$fixture/race-bin/bash.tmp" "$fixture/race-bin/bash"
chmod +x "$fixture/race-bin/bash"
EVIDENCE_REAL_BASH="$BASH" EVIDENCE_RACE_KIND="$kind" \
    EVIDENCE_RACE_CONSUMER="$consumer" PATH="$fixture/race-bin:$PATH" \
    choose_evidence compact "$fixture/race"
archive "$fixture/race" "$fixture/race.tar.gz"
mkdir "$fixture/race-unpacked"
tar -xzf "$fixture/race.tar.gz" -C "$fixture/race-unpacked"
[[ -d "$fixture/race-unpacked/$stored_path" && ! -e "$fixture/race/$kind/selection.txt" ]] || exit 1
rm "$consumer/.tools/$kind"
ln -s "$active" "$consumer/.tools/$kind"
rmdir "$consumer/.tools/$kind-set.evidenceRace"

# Run the actual composite-action collector and check its original logs too.
yq -o json '.' "$ROOT/.github/actions/retain-failure-evidence/action.yml" |
    jq -er '.runs.steps[] | select(.id == "archive") | .run' > "$fixture/collector.sh"
mkdir "$fixture/action-temp"
printf 'original_status=23\n' > "$fixture/action-temp/portable-regression.log"
EVIDENCE_TEMP_ROOT="$fixture/action-temp" EVIDENCE_REPOSITORY_ROOT="$consumer" \
    EVIDENCE_ACTION_ROOT="$ROOT/.github/actions/retain-failure-evidence" \
    EVIDENCE_COMPACT=true EVIDENCE_HOST_VERSIONS="$pins" EVIDENCE_IC_PINS="$pins" \
    GITHUB_OUTPUT="$fixture/action-output" RUNNER_TEMP="$fixture/action-temp" \
    "$BASH" --noprofile --norc -e -o pipefail "$fixture/collector.sh"
output="$(sed -n 's/^path=//p' "$fixture/action-output")"
mkdir "$fixture/action-unpacked"
tar -xzf "$output" -C "$fixture/action-unpacked"
cmp "$fixture/action-temp/portable-regression.log" "$fixture/action-unpacked/portable-regression.log"
[[ ! -e "$fixture/action-unpacked/.tools/$active" ]] || exit 1
cmp "$candidate/payload" "$fixture/action-unpacked/.tools/$kind-set.evidenceCandidate/payload"

# A rejected store must not prevent uploading independent local evidence.
mkdir -p "$fixture/rejected/consumer/.tools/host-set.failed" "$fixture/rejected/consumer/.tools/rust/build" "$fixture/rejected/outside"
printf 'local failed payload\n' > "$fixture/rejected/consumer/.tools/host-set.failed/payload"
printf 'local failed build\n' > "$fixture/rejected/consumer/.tools/rust/build/payload"
printf 'never collect this\n' > "$fixture/rejected/outside/private"
ln -s "$fixture/rejected/outside" "$fixture/rejected/store-link"
mkdir "$fixture/rejected/writable-store"
chmod 0777 "$fixture/rejected/writable-store"
ln -s "$fixture/rejected/outside" "$fixture/rejected/consumer/.tools/ic-set.reference"
yq -o json '.' "$ROOT/.github/actions/retain-failure-evidence/action.yml" |
    jq -er '.runs.steps[] | select(.id == "completeness") | .run' > "$fixture/completeness.sh"
for bad_store in "$fixture/rejected/store-link" relative-store "$fixture/rejected/new/./store" "$fixture/rejected/writable-store"; do
    for compact in true false; do
        attempt="$(mktemp -d "$fixture/rejected/attempt.XXXXXX")"
        printf 'original setup failure\n' > "$attempt/tools-install.log"
        SHARED_TOOLING_STORE="$bad_store" EVIDENCE_TEMP_ROOT="$attempt" \
            EVIDENCE_REPOSITORY_ROOT="$fixture/rejected/consumer" \
            EVIDENCE_ACTION_ROOT="$ROOT/.github/actions/retain-failure-evidence" \
            EVIDENCE_COMPACT="$compact" GITHUB_OUTPUT="$attempt/output" RUNNER_TEMP="$attempt" \
            "$BASH" --noprofile --norc -e -o pipefail "$fixture/collector.sh" > "$attempt/collector.log" 2>&1
        grep -qx 'collection-complete=false' "$attempt/output"
        output="$(sed -n 's/^path=//p' "$attempt/output")"
        mkdir "$attempt/unpacked"
        tar -xzf "$output" -C "$attempt/unpacked"
        cmp "$attempt/tools-install.log" "$attempt/unpacked/tools-install.log"
        for local_file in host-set.failed/payload rust/build/payload; do
            cmp "$fixture/rejected/consumer/.tools/$local_file" "$attempt/unpacked/.tools/$local_file"
        done
        [[ -L "$attempt/unpacked/.tools/ic-set.reference" &&
           -z "$(find "$attempt/unpacked" -type f -name private)" ]] || exit 1
        detail=("$attempt/unpacked"/tool-evidence.*)
        [[ ${#detail[@]} == 1 && "$(cat "${detail[0]}/selection-status")" != 0 &&
           -s "${detail[0]}/selection.log" ]] || exit 1
        if "$BASH" -e "$fixture/completeness.sh" > "$attempt/completeness.log" 2>&1; then
            echo 'incomplete collection was reported as success' >&2; exit 1
        fi
    done
done
rm -rf "$candidate"
echo 'Successful tool compaction, failed verification, unknown selections and actual collector passed'
fixture_complete=true
