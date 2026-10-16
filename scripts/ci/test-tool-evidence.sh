#!/usr/bin/env bash
# Shared companions: scripts/ci/select-tool-evidence.sh scripts/ci/archive-evidence.sh .github/actions/retain-failure-evidence/action.yml scripts/ci/fixture-lifecycle.sh
set -euo pipefail
[[ $# == 3 ]] || { echo 'usage: test-tool-evidence.sh CONSUMER host|ic CALLER-PINS' >&2; exit 2; }
ROOT="${BASH_SOURCE[0]}"
[[ "$ROOT" == /* ]] || ROOT="$PWD/$ROOT"
ROOT="$(cd -P "${ROOT%/*}/../.." && printf '%s/.' "$PWD")"
ROOT="${ROOT%/.}"
consumer="$1"; kind="$2"; pins="$3"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/tool-evidence-test.XXXXXX")"
fixture="$(CDPATH="" cd -P -- "$fixture" && printf '%s/.' "$PWD")"
fixture="${fixture%/.}"
# shellcheck source=scripts/ci/fixture-lifecycle.sh
source "$ROOT/scripts/ci/fixture-lifecycle.sh"
fixture_complete=false
trap 'fixture_finish "$?" "$fixture" "Tool evidence fixture retained"' EXIT
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
# With no exception inputs, the actual collector uses catalogs beside its action,
# even when the inspected repository carries different or missing catalogs.
snapshot="$fixture/snapshot"
mkdir -p "$snapshot/.github/actions/retain-failure-evidence" "$snapshot/ci" "$fixture/default-temp"
ln -s "$ROOT/scripts" "$snapshot/scripts"
cp "$pins" "$snapshot/ci/tool-versions.env"
cp "$pins" "$snapshot/ci/ic-tools.tsv"
EVIDENCE_TEMP_ROOT="$fixture/default-temp" EVIDENCE_REPOSITORY_ROOT="$consumer" \
    EVIDENCE_ACTION_ROOT="$snapshot/.github/actions/retain-failure-evidence" \
    EVIDENCE_COMPACT=true EVIDENCE_HOST_VERSIONS='' EVIDENCE_IC_PINS='' \
    GITHUB_OUTPUT="$fixture/default-output" RUNNER_TEMP="$fixture/default-temp" \
    "$BASH" --noprofile --norc -e -o pipefail "$fixture/collector.sh"
default_details=("$fixture/default-temp"/tool-evidence.*/"$kind")
[[ ${#default_details[@]} == 1 && -f "${default_details[0]}/selection.txt" ]] || exit 1
cmp "$pins" "${default_details[0]}/caller-pins"
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
# Caller-prepared evidence bypasses collection and preserves available files,
# including the original log when archive creation failed. No tool roots exist.
prepared="$fixture/prepared evidence"
mkdir "$prepared"
printf 'archive bytes\n' > "$prepared/original.tar.gz"
printf 'original failure\n' > "$prepared/original.log"
ln -s original.tar.gz "$prepared/archive-link"
ln -s original.log "$prepared/log-link"
for scenario in complete missing-archive missing-log missing-both archive-link log-link relative empty newline carriage glob same; do
    archive="$prepared/original.tar.gz"; log="$prepared/original.log"
    expected_complete=false; expected_status=0
    : > "$prepared/expected-paths"
    case "$scenario" in
        complete) expected_complete=true; printf '%s\n' "$archive" "$log" > "$prepared/expected-paths" ;;
        missing-archive) archive="$prepared/absent"; printf '%s\n' "$log" > "$prepared/expected-paths" ;;
        missing-log) log="$prepared/absent"; printf '%s\n' "$archive" > "$prepared/expected-paths" ;;
        missing-both) archive="$prepared/absent-archive"; log="$prepared/absent-log" ;;
        archive-link) archive="$prepared/archive-link"; printf '%s\n' "$log" > "$prepared/expected-paths" ;;
        log-link) log="$prepared/log-link"; printf '%s\n' "$archive" > "$prepared/expected-paths" ;;
        relative) archive=relative.tar.gz; expected_status=2 ;;
        empty) log=''; expected_status=2 ;;
        newline) archive="$prepared/"$'bad\npath'; expected_status=2 ;;
        carriage) archive="$prepared/"$'bad\rpath'; expected_status=2 ;;
        glob) archive="$prepared/*.tar.gz"; expected_status=2 ;;
        same) log="$archive"; expected_status=2 ;;
    esac
    output="$prepared/$scenario.output"; : > "$output"
    status=0
    EVIDENCE_PREPARED_ARCHIVE="$archive" EVIDENCE_COLLECTION_LOG="$log" \
        EVIDENCE_TEMP_ROOT='' EVIDENCE_REPOSITORY_ROOT='' EVIDENCE_ACTION_ROOT="$prepared/absent" \
        GITHUB_OUTPUT="$output" "$BASH" --noprofile --norc -e -o pipefail "$fixture/collector.sh" \
        > "$prepared/$scenario.log" 2>&1 || status=$?
    [[ "$status" == "$expected_status" ]] || exit 1
    if [[ "$status" != 0 ]]; then [[ ! -s "$output" ]] || exit 1; continue; fi
    grep -Fx "collection-complete=$expected_complete" "$output" > /dev/null
    sed -n '/^path<<EVIDENCE_PATHS$/,/^EVIDENCE_PATHS$/{ /^path<</d; /^EVIDENCE_PATHS$/d; p; }' \
        "$output" > "$prepared/actual-paths"
    cmp "$prepared/expected-paths" "$prepared/actual-paths"
done
# Optional action inputs serve prepared uploads only. Ordinary collection still
# requires both roots and must refuse before sourcing helpers or creating output.
for missing in temp repository both; do
    collection_temp="$prepared" collection_repository="$prepared"
    [[ "$missing" == repository ]] || collection_temp=''
    [[ "$missing" == temp ]] || collection_repository=''
    output="$prepared/missing-$missing.output"; : > "$output"
    status=0
    EVIDENCE_PREPARED_ARCHIVE='' EVIDENCE_COLLECTION_LOG='' \
        EVIDENCE_TEMP_ROOT="$collection_temp" EVIDENCE_REPOSITORY_ROOT="$collection_repository" \
        EVIDENCE_ACTION_ROOT="$prepared/absent" GITHUB_OUTPUT="$output" \
        "$BASH" --noprofile --norc -e -o pipefail "$fixture/collector.sh" \
        > "$prepared/missing-$missing.log" 2>&1 || status=$?
    [[ "$status" == 2 && ! -s "$output" ]] || exit 1
done
# Always upload available prepared diagnostics before enforcing completeness;
# native collection retains its existing cancellation boundary.
# shellcheck disable=SC2016 # Compare literal GitHub expressions.
yq -o json '.' "$ROOT/.github/actions/retain-failure-evidence/action.yml" | jq -e \
    --arg guard "inputs.prepared-archive != '' || inputs.collection-log != '' || !cancelled()" \
    --arg nonempty "steps.archive.outputs.path != ''" \
    --arg incomplete "steps.archive.outputs.collection-complete == 'false'" '
    .runs.steps as $steps |
    $steps[0].if == ("${{ always() && (" + $guard + ") }}") and
    $steps[1].if == ("${{ always() && " + $nonempty + " && (" + $guard + ") }}") and
    $steps[2].if == ("${{ always() && " + $incomplete + " && (" + $guard + ") }}") and
    $steps[1].with["compression-level"] == 0 and $steps[1].with.path == "${{ steps.archive.outputs.path }}"
' > /dev/null
rm -rf "$candidate"
echo 'Successful tool compaction, failed verification, unknown selections and actual collector passed'
fixture_complete=true
