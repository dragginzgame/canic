#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd -P)"
FIXTURE="$(mktemp -d "${TMPDIR:-/tmp}/shared-tooling-snapshot-test.XXXXXX")"
trap 'if [[ $? == 0 ]]; then rm -rf "$FIXTURE"; else printf "Failed snapshot-distribution fixture retained: %s\n" "$FIXTURE" >&2; fi' EXIT
# Match refresh-consumer physical paths when TMPDIR contains a host alias.
FIXTURE="$(cd "$FIXTURE" && pwd -P)"
REAL_GIT="$(command -v git)"
export REAL_GIT

source_root="$FIXTURE/source"
consumer_root="$FIXTURE/consumer"
mkdir -p \
    "$source_root/scripts/ci" \
    "$source_root/scripts/distribution" \
    "$consumer_root" \
    "$FIXTURE/bin"
git init -q "$consumer_root"

cp "$ROOT/scripts/ci/verify-file-checksum.sh" "$source_root/scripts/ci/"
cp "$ROOT/scripts/ci/verify-shared-tooling-snapshot.sh" "$source_root/scripts/ci/"
cp "$ROOT/scripts/distribution/refresh-consumer.sh" "$source_root/scripts/distribution/"
printf '#!/usr/bin/env bash\nprintf "sample tool\\n"\n' >"$source_root/scripts/ci/sample.sh"
chmod +x "$source_root/scripts/ci/sample.sh"

# Freeze the fake revision independently of the source working tree.
revision_root="$FIXTURE/revision"
mkdir -p "$revision_root"
cp -Rp "$source_root/scripts" "$revision_root/"

cat >"$FIXTURE/bin/git" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail

if [[ "$1" != "-C" || "$#" -lt 4 ]]; then
    exit 2
fi

repository="$2"
if [[ "$repository" != "$SNAPSHOT_TEST_SOURCE_ROOT" ]]; then
    if [[ "${SNAPSHOT_TEST_STATUS_FAIL:-}" == true && "$3" == --literal-pathspecs ]]; then exit 9; fi
    exec "$REAL_GIT" "$@"
fi
shift 2

case "$1:$2" in
rev-parse:--show-toplevel)
    printf '%s\n' "$repository"
    ;;
rev-parse:HEAD)
    printf '%s\n' 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
    ;;
status:--porcelain)
    if [[ "${SNAPSHOT_TEST_DIRTY:-}" == "true" ]]; then
        printf ' M scripts/ci/sample.sh\n'
    fi
    ;;
remote:get-url)
    printf '%s\n' 'git@github.com:dragginzgame/shared-tooling.git'
    ;;
ls-tree:-z)
    path="$5"
    committed_file="$SNAPSHOT_TEST_REVISION_ROOT/$path"
    if [[ -f "$committed_file" ]]; then
        mode=100644
        [[ ! -x "$committed_file" ]] || mode=100755
        printf '%s blob bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\t%s\0' "$mode" "$path"
    fi
    ;;
cat-file:blob)
    path="${3#*:}"
    if [[ "${SNAPSHOT_TEST_SOURCE_DRIFT:-}" == "true" && "$path" == "scripts/ci/sample.sh" ]]; then
        printf '# concurrent source edit\n' >>"$repository/$path"
        chmod -x "$repository/$path"
    fi
    cat "$SNAPSHOT_TEST_REVISION_ROOT/$path"
    ;;
*)
    exit 2
    ;;
esac
SCRIPT
chmod +x "$FIXTURE/bin/git"
export SNAPSHOT_TEST_REVISION_ROOT="$revision_root"
export SNAPSHOT_TEST_SOURCE_ROOT="$source_root"

printf 'ignored working-tree content\n' >"$source_root/ignored.txt"
if PATH="$FIXTURE/bin:$PATH" \
    bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" \
    --consumer "$consumer_root" \
    --file scripts/ci/verify-file-checksum.sh \
    --file scripts/ci/verify-shared-tooling-snapshot.sh \
    --file ignored.txt >/dev/null 2>&1; then
    echo "snapshot distribution test failed: a file absent from the source revision was accepted" >&2
    exit 1
fi
[[ ! -e "$consumer_root/.shared-tooling.snapshot" ]]
[[ ! -e "$consumer_root/scripts" ]]
[[ ! -e "$consumer_root/ignored.txt" ]]

if PATH="$FIXTURE/bin:$PATH" \
    bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" \
    --consumer "$consumer_root" \
    --file scripts/ci/sample.sh >/dev/null 2>&1; then
    echo "snapshot distribution test failed: incomplete verifier set was accepted" >&2
    exit 1
fi
[[ ! -e "$consumer_root/.shared-tooling.snapshot" ]]
[[ ! -e "$consumer_root/scripts/ci/sample.sh" ]]

if ! SNAPSHOT_TEST_SOURCE_DRIFT=true PATH="$FIXTURE/bin:$PATH" \
    bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" \
    --consumer "$consumer_root" \
    --file scripts/ci/verify-file-checksum.sh \
    --file scripts/ci/verify-shared-tooling-snapshot.sh \
    --file scripts/ci/sample.sh >"$FIXTURE/initial-refresh.log" 2>&1; then
    cat "$FIXTURE/initial-refresh.log" >&2
    exit 1
fi

cmp "$revision_root/scripts/ci/sample.sh" "$consumer_root/scripts/ci/sample.sh"
[[ -x "$consumer_root/scripts/ci/sample.sh" ]]
[[ ! -x "$source_root/scripts/ci/sample.sh" ]]
if cmp -s "$source_root/scripts/ci/sample.sh" "$consumer_root/scripts/ci/sample.sh"; then
    echo "snapshot distribution test failed: concurrent source drift reached the consumer" >&2
    exit 1
fi

bash "$consumer_root/scripts/ci/verify-shared-tooling-snapshot.sh" \
    --consumer "$consumer_root" >/dev/null

# The checksum helper is inspected data, never verification authority. Neither
# helper-only corruption nor corruption of both helper and payload may pass.
export SNAPSHOT_HELPER_EXECUTED="$FIXTURE/helper-executed"
for corruption in helper both; do
    cat > "$consumer_root/scripts/ci/verify-file-checksum.sh" <<'SCRIPT'
#!/usr/bin/env bash
: > "$SNAPSHOT_HELPER_EXECUTED"
exit 0
SCRIPT
    if [[ "$corruption" == both ]]; then
        printf '# changed payload\n' >> "$consumer_root/scripts/ci/sample.sh"
    fi
    if bash "$ROOT/scripts/ci/verify-shared-tooling-snapshot.sh" --consumer "$consumer_root" \
        > "$FIXTURE/corrupt-$corruption.log" 2>&1; then exit 1; fi
    [[ ! -e "$SNAPSHOT_HELPER_EXECUTED" ]]
    cp -p "$revision_root/scripts/ci/verify-file-checksum.sh" "$consumer_root/scripts/ci/"
    cp -p "$revision_root/scripts/ci/sample.sh" "$consumer_root/scripts/ci/"
done

# Exercise the independent bootstrap on both backends; output that looks valid
# must not hide failure, and a failed selected backend must not fall back.
hash_bin="$FIXTURE/hash-bin"
mkdir "$hash_bin"
ln -s "$(command -v dirname)" "$hash_bin/dirname"
export SNAPSHOT_REAL_SHASUM
SNAPSHOT_REAL_SHASUM="$(command -v shasum)"
export SNAPSHOT_FALLBACK_USED="$FIXTURE/fallback-used"
printf '#!%s\n' "$BASH" > "$hash_bin/shasum"
cat >> "$hash_bin/shasum" <<'SCRIPT'
: > "$SNAPSHOT_FALLBACK_USED"
exec "$SNAPSHOT_REAL_SHASUM" "$@"
SCRIPT
chmod +x "$hash_bin/shasum"
PATH="$hash_bin" "$BASH" "$ROOT/scripts/ci/verify-shared-tooling-snapshot.sh" \
    --consumer "$consumer_root" > "$FIXTURE/shasum.log"
[[ -f "$SNAPSHOT_FALLBACK_USED" ]]
rm "$SNAPSHOT_FALLBACK_USED"
printf '#!%s\n' "$BASH" > "$hash_bin/sha256sum"
cat >> "$hash_bin/sha256sum" <<'SCRIPT'
case "${SNAPSHOT_HASH_MODE:-valid}" in
    malformed) printf 'invalid digest\n' ;;
    failed) "$SNAPSHOT_REAL_SHASUM" -a 256; exit 17 ;;
    *) exec "$SNAPSHOT_REAL_SHASUM" -a 256 ;;
esac
SCRIPT
chmod +x "$hash_bin/sha256sum"
PATH="$hash_bin" "$BASH" "$ROOT/scripts/ci/verify-shared-tooling-snapshot.sh" \
    --consumer "$consumer_root" > "$FIXTURE/sha256sum.log"
for failure in malformed failed; do
    if SNAPSHOT_HASH_MODE="$failure" PATH="$hash_bin" "$BASH" \
        "$ROOT/scripts/ci/verify-shared-tooling-snapshot.sh" --consumer "$consumer_root" \
        > "$FIXTURE/hash-$failure.log" 2>&1; then exit 1; fi
done
[[ ! -e "$SNAPSHOT_FALLBACK_USED" ]]

cp "$consumer_root/.shared-tooling.snapshot" "$consumer_root/duplicate.snapshot"
awk '$1 == "file" { print; exit }' "$consumer_root/.shared-tooling.snapshot" \
    >>"$consumer_root/duplicate.snapshot"
if bash "$consumer_root/scripts/ci/verify-shared-tooling-snapshot.sh" \
    --consumer "$consumer_root" --manifest duplicate.snapshot \
    >"$FIXTURE/duplicate.log" 2>&1; then
    echo "snapshot distribution test failed: duplicate file record was accepted" >&2
    exit 1
fi
printf '# drift\n' >>"$consumer_root/scripts/ci/sample.sh"
if bash "$consumer_root/scripts/ci/verify-shared-tooling-snapshot.sh" \
    --consumer "$consumer_root" >/dev/null 2>&1; then
    echo "snapshot distribution test failed: consumer drift was accepted" >&2
    exit 1
fi

cp "$consumer_root/scripts/ci/sample.sh" "$FIXTURE/dirty-sample"
cp "$consumer_root/.shared-tooling.snapshot" "$FIXTURE/saved-manifest"
if PATH="$FIXTURE/bin:$PATH" \
    bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" \
    --consumer "$consumer_root" >"$FIXTURE/dirty-consumer.log" 2>&1; then
    echo 'snapshot distribution test failed: untracked consumer edits were overwritten' >&2
    exit 1
fi
cmp "$FIXTURE/dirty-sample" "$consumer_root/scripts/ci/sample.sh"
cmp "$FIXTURE/saved-manifest" "$consumer_root/.shared-tooling.snapshot"
rg -F 'consumer destination has local changes' "$FIXTURE/dirty-consumer.log" >/dev/null
cp -p "$revision_root/scripts/ci/sample.sh" "$consumer_root/scripts/ci/sample.sh"
# Unrelated edits and a retry of the exact already-installed bytes are allowed.
printf 'unrelated work\n' > "$consumer_root/unrelated.txt"
PATH="$FIXTURE/bin:$PATH" \
    bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" \
    --consumer "$consumer_root" >/dev/null
bash "$consumer_root/scripts/ci/verify-shared-tooling-snapshot.sh" \
    --consumer "$consumer_root" >/dev/null

chmod -x "$consumer_root/scripts/ci/sample.sh"
if bash "$consumer_root/scripts/ci/verify-shared-tooling-snapshot.sh" \
    --consumer "$consumer_root" >/dev/null 2>&1; then
    echo "snapshot distribution test failed: executable-mode drift was accepted" >&2
    exit 1
fi
if PATH="$FIXTURE/bin:$PATH" \
    bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" --consumer "$consumer_root" >/dev/null 2>&1; then
    echo 'snapshot distribution test failed: local executable-mode change was overwritten' >&2
    exit 1
fi
[[ ! -x "$consumer_root/scripts/ci/sample.sh" ]]
chmod +x "$consumer_root/scripts/ci/sample.sh"

# Ignored destinations are still local work, not disposable output.
mkdir -p "$consumer_root/.git/info"
printf 'scripts/ci/sample.sh\n' > "$consumer_root/.git/info/exclude"
printf '# ignored local edit\n' >> "$consumer_root/scripts/ci/sample.sh"
cp "$consumer_root/scripts/ci/sample.sh" "$FIXTURE/ignored-sample"
if PATH="$FIXTURE/bin:$PATH" \
    bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" --consumer "$consumer_root" >/dev/null 2>&1; then
    echo 'snapshot distribution test failed: ignored consumer file was overwritten' >&2
    exit 1
fi
cmp "$FIXTURE/ignored-sample" "$consumer_root/scripts/ci/sample.sh"
cp -p "$revision_root/scripts/ci/sample.sh" "$consumer_root/scripts/ci/sample.sh"

if SNAPSHOT_TEST_DIRTY=true PATH="$FIXTURE/bin:$PATH" \
    bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" \
    --consumer "$consumer_root" >/dev/null 2>&1; then
    echo "snapshot distribution test failed: dirty source was accepted" >&2
    exit 1
fi

# A custom manifest can create its own parent directories on the first refresh.
nested_consumer="$FIXTURE/nested consumer"
mkdir -p "$nested_consumer"
git init -q "$nested_consumer"
PATH="$FIXTURE/bin:$PATH" \
    bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" --consumer "$nested_consumer" \
    --manifest 'config/shared tools/snapshot' \
    --file scripts/ci/verify-file-checksum.sh \
    --file scripts/ci/verify-shared-tooling-snapshot.sh >/dev/null
PATH="$FIXTURE/bin:$PATH" \
    bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" --consumer "$nested_consumer" \
    --manifest 'config/shared tools/snapshot' >/dev/null
bash "$nested_consumer/scripts/ci/verify-shared-tooling-snapshot.sh" \
    --consumer "$nested_consumer" --manifest 'config/shared tools/snapshot' >/dev/null

# Failure to create that parent must precede any snapshot file replacement.
blocked_consumer="$FIXTURE/blocked-consumer"
mkdir -p "$blocked_consumer/scripts/ci"
git init -q "$blocked_consumer"
printf 'not a directory\n' >"$blocked_consumer/config"
# Match the selected payload so the destination-change guard cannot mask the
# parent-directory failure this fixture is intended to test.
cp -p "$revision_root/scripts/ci/verify-file-checksum.sh" "$blocked_consumer/scripts/ci/verify-file-checksum.sh"
cp "$blocked_consumer/scripts/ci/verify-file-checksum.sh" "$FIXTURE/original-checksum.sh"
if PATH="$FIXTURE/bin:$PATH" \
    bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" --consumer "$blocked_consumer" \
    --manifest config/shared.snapshot \
    --file scripts/ci/verify-file-checksum.sh \
    --file scripts/ci/verify-shared-tooling-snapshot.sh >/dev/null 2>&1; then
    echo "snapshot distribution test failed: a blocked manifest parent was accepted" >&2
    exit 1
fi
cmp "$FIXTURE/original-checksum.sh" "$blocked_consumer/scripts/ci/verify-file-checksum.sh"
[[ ! -e "$blocked_consumer/scripts/ci/verify-shared-tooling-snapshot.sh" ]]

# Real Git status must distinguish index changes from working-tree changes.
# Clone existing history; these fixtures never create commits or touch its index.
tracked_consumer="$FIXTURE/tracked consumer"
git clone --quiet --shared "$ROOT" "$tracked_consumer"
# A consumer checkout already carries a default snapshot. Exercise that state
# upstream too, preserving its bytes and using only our own initial manifest.
if [[ ! -e "$tracked_consumer/.shared-tooling.snapshot" ]]; then
    cp "$consumer_root/.shared-tooling.snapshot" "$tracked_consumer/.shared-tooling.snapshot"
fi
cp "$tracked_consumer/.shared-tooling.snapshot" "$FIXTURE/existing-consumer-manifest"
tracked_manifest_dir="$(mktemp -d "$tracked_consumer/.snapshot-fixture.XXXXXX")"
tracked_manifest="${tracked_manifest_dir#"$tracked_consumer/"}/snapshot"
checksum_path=scripts/ci/verify-file-checksum.sh
verifier_path=scripts/ci/verify-shared-tooling-snapshot.sh
cp "$tracked_consumer/$checksum_path" "$FIXTURE/original-tracked-checksum"
for path in "$checksum_path" "$verifier_path"; do
    printf '\n# Upstream fixture update.\n' >> "$revision_root/$path"
done
for state in staged unstaged deleted unavailable; do
    case "$state" in
        staged)
            replacement="$(git -C "$tracked_consumer" rev-parse HEAD:README.md)"
            git -C "$tracked_consumer" update-index --cacheinfo "100755,$replacement,$verifier_path"
            git -C "$tracked_consumer" diff --quiet HEAD -- "$verifier_path"
            if git -C "$tracked_consumer" diff --cached --quiet HEAD -- "$verifier_path"; then
                echo 'snapshot distribution test failed: hidden staged-change fixture is invalid' >&2
                exit 1
            fi
            ;;
        unstaged) printf '\n# Consumer edit.\n' >> "$tracked_consumer/$verifier_path" ;;
        deleted) rm "$tracked_consumer/$verifier_path" ;;
        unavailable) export SNAPSHOT_TEST_STATUS_FAIL=true ;;
    esac
    index_before="$(git -C "$tracked_consumer" write-tree)"
    if [[ -e "$tracked_consumer/$verifier_path" ]]; then
        cp "$tracked_consumer/$verifier_path" "$FIXTURE/before-verifier"
    fi
    if PATH="$FIXTURE/bin:$PATH" \
        bash "$source_root/scripts/distribution/refresh-consumer.sh" \
        --source "$source_root" --consumer "$tracked_consumer" \
        --manifest "$tracked_manifest" \
        --file "$checksum_path" --file "$verifier_path" > "$FIXTURE/$state.log" 2>&1; then
        echo "snapshot distribution test failed: $state consumer state was overwritten" >&2
        exit 1
    fi
    unset SNAPSHOT_TEST_STATUS_FAIL
    case "$state" in
        unavailable) rg -F 'cannot inspect consumer changes' "$FIXTURE/$state.log" >/dev/null ;;
        *) rg -F 'consumer destination has local changes' "$FIXTURE/$state.log" >/dev/null ;;
    esac
    cmp "$FIXTURE/original-tracked-checksum" "$tracked_consumer/$checksum_path"
    [[ "$(git -C "$tracked_consumer" write-tree)" == "$index_before" ]]
    [[ ! -e "$tracked_consumer/$tracked_manifest" ]]
    cmp "$FIXTURE/existing-consumer-manifest" "$tracked_consumer/.shared-tooling.snapshot"
    if [[ "$state" == deleted ]]; then
        [[ ! -e "$tracked_consumer/$verifier_path" ]]
    else
        cmp "$FIXTURE/before-verifier" "$tracked_consumer/$verifier_path"
    fi
    git -C "$tracked_consumer" read-tree HEAD
    git -C "$tracked_consumer" checkout-index --force -- "$verifier_path"
done
# A clean selected file can update while unrelated tracked edits survive.
printf '\nUnrelated consumer edit.\n' >> "$tracked_consumer/README.md"
cp "$tracked_consumer/README.md" "$FIXTURE/consumer-readme"
PATH="$FIXTURE/bin:$PATH" \
    bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" --consumer "$tracked_consumer" \
    --manifest "$tracked_manifest" \
    --file "$checksum_path" --file "$verifier_path" >/dev/null
cmp "$FIXTURE/consumer-readme" "$tracked_consumer/README.md"
cmp "$revision_root/$checksum_path" "$tracked_consumer/$checksum_path"
cmp "$revision_root/$verifier_path" "$tracked_consumer/$verifier_path"
cmp "$FIXTURE/existing-consumer-manifest" "$tracked_consumer/.shared-tooling.snapshot"
bash "$ROOT/scripts/ci/verify-shared-tooling-snapshot.sh" \
    --consumer "$tracked_consumer" --manifest "$tracked_manifest" >/dev/null

# Export the documented governance selection, then check links in that export.
# This proves closure of the consumer file set, not merely the source checkout.
governance_consumer="$FIXTURE/governance"
mkdir "$governance_consumer"
git init -q "$governance_consumer"
governance_args=()
governance_docs=()
while IFS= read -r path; do
    [[ -n "$path" ]] || exit 1
    mkdir -p "$source_root/$(dirname "$path")" "$revision_root/$(dirname "$path")"
    cp -p "$ROOT/$path" "$source_root/$path"
    cp -p "$ROOT/$path" "$revision_root/$path"
    governance_args+=(--file "$path")
    case "$path" in *.md) governance_docs+=("$path") ;; esac
done < "$ROOT/scripts/distribution/governance-files.txt"
PATH="$FIXTURE/bin:$PATH" bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" --consumer "$governance_consumer" "${governance_args[@]}" \
    > "$FIXTURE/governance-refresh.log"
perl "$ROOT/scripts/ci/check-documentation-links.pl" --root "$governance_consumer" \
    "${governance_docs[@]}" > "$FIXTURE/governance-links.log"
# Ensure the checker actually inspects exported targets, not the source files.
rm "$governance_consumer/docs/tag-maintenance.md"
if perl "$ROOT/scripts/ci/check-documentation-links.pl" --root "$governance_consumer" \
    "${governance_docs[@]}" > "$FIXTURE/governance-missing.log" 2>&1; then exit 1; fi

echo "snapshot distribution test passed"
