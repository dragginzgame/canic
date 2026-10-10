#!/usr/bin/env bash
set -euo pipefail

ROOT="$0"
[[ "$ROOT" == /* ]] || ROOT="$PWD/$ROOT"
ROOT="$(cd -P "${ROOT%/*}/../.." && printf '%s/.' "$PWD")"
ROOT="${ROOT%/.}"
FIXTURE="$(mktemp -d "${TMPDIR:-/tmp}/shared-tooling-snapshot-test.XXXXXX")"
# Bash 3.2 can enter EXIT with status zero after nounset; require completion too.
fixture_complete=false
finish() {
    local status=$?
    [[ "$fixture_complete" == true || "$status" != 0 ]] || status=1
    if [[ "$status" == 0 ]]; then rm -rf "$FIXTURE"
    else printf "Failed snapshot-distribution fixture retained: %s\n" "$FIXTURE" >&2; fi
    exit "$status"
}
trap finish EXIT
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
printf '8.8.8\n' > "$consumer_root/VERSION"

cp "$ROOT/scripts/ci/verify-file-checksum.sh" "$source_root/scripts/ci/"
cp "$ROOT/scripts/ci/verify-shared-tooling-snapshot.sh" "$source_root/scripts/ci/"
cp "$ROOT/scripts/distribution/refresh-consumer.sh" "$source_root/scripts/distribution/"
printf '#!/usr/bin/env bash\nprintf "sample tool\\n"\n' >"$source_root/scripts/ci/sample.sh"
chmod +x "$source_root/scripts/ci/sample.sh"

# Freeze the fake revision independently of the source working tree.
revision_root="$FIXTURE/revision"
mkdir -p "$revision_root"
cp -Rp "$source_root/scripts" "$revision_root/"
printf '0.2.8\n' > "$revision_root/VERSION"
# The export must use the committed version even if working bytes differ.
printf '9.9.9\n' > "$source_root/VERSION"

cat >"$FIXTURE/bin/git" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail

if [[ "$1" != "-C" || "$#" -lt 4 ]]; then
    exit 2
fi

repository="$2"
if [[ "$repository" != "$SNAPSHOT_TEST_SOURCE_ROOT" ]]; then
    if [[ "${SNAPSHOT_TEST_STATUS_FAIL:-}" == true && "$3" == --literal-pathspecs && "$4" == status ]]; then exit 9; fi
    exec "$REAL_GIT" "$@"
fi
shift 2

case "$1:$2" in
rev-parse:--show-toplevel)
    [[ "${SNAPSHOT_TEST_TOP_FAIL:-}" != true ]] || exit 9
    printf '%s%s\n' "$repository" "${SNAPSHOT_TEST_TOP_SUFFIX:-}"
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
[[ ! -e "$consumer_root/.shared-tooling.snapshot" ]] || exit 1
[[ ! -e "$consumer_root/scripts" ]] || exit 1
[[ ! -e "$consumer_root/ignored.txt" ]] || exit 1

if PATH="$FIXTURE/bin:$PATH" \
    bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" \
    --consumer "$consumer_root" \
    --file scripts/ci/sample.sh >/dev/null 2>&1; then
    echo "snapshot distribution test failed: incomplete verifier set was accepted" >&2
    exit 1
fi
[[ ! -e "$consumer_root/.shared-tooling.snapshot" ]] || exit 1
[[ ! -e "$consumer_root/scripts/ci/sample.sh" ]] || exit 1

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
[[ -x "$consumer_root/scripts/ci/sample.sh" ]] || exit 1
[[ ! -x "$source_root/scripts/ci/sample.sh" ]] || exit 1
if cmp -s "$source_root/scripts/ci/sample.sh" "$consumer_root/scripts/ci/sample.sh"; then
    echo "snapshot distribution test failed: concurrent source drift reached the consumer" >&2
    exit 1
fi

bash "$consumer_root/scripts/ci/verify-shared-tooling-snapshot.sh" \
    --consumer "$consumer_root" >/dev/null

# Unsupported directory names must refuse, never select a trimmed neighbor.
# Keep valid snapshot bytes in both neighbors so missing inputs cannot mask the
# check. A normal-looking symlink must not hide a forbidden physical root.
cp "$consumer_root/.shared-tooling.snapshot" "$FIXTURE/path-manifest"
for suffix in $'\n' $'\n\n' $'\r' $'\r\n' $'\nchild'; do
    bad_consumer="$consumer_root$suffix"
    bad_source="$source_root$suffix"
    mkdir "$bad_consumer" "$bad_source"
    git init -q "$bad_consumer"
    cp -Rp "$consumer_root/scripts" "$bad_consumer/"
    cp "$FIXTURE/path-manifest" "$bad_consumer/.shared-tooling.snapshot"
    ln -s "$bad_consumer" "$FIXTURE/consumer-alias"
    ln -s "$bad_source" "$FIXTURE/source-alias"
    for candidate in "$bad_consumer" "$FIXTURE/consumer-alias"; do
        if bash "$ROOT/scripts/ci/verify-shared-tooling-snapshot.sh" --consumer "$candidate" \
            > "$FIXTURE/path-verify.log" 2>&1; then
            echo 'snapshot verifier accepted a forbidden directory name' >&2; exit 1
        fi
        if PATH="$FIXTURE/bin:$PATH" bash "$source_root/scripts/distribution/refresh-consumer.sh" \
            --source "$source_root" --consumer "$candidate" > "$FIXTURE/path-refresh.log" 2>&1; then
            echo 'snapshot refresh accepted a forbidden consumer directory' >&2; exit 1
        fi
    done
    for candidate in "$bad_source" "$FIXTURE/source-alias"; do
        if PATH="$FIXTURE/bin:$PATH" bash "$source_root/scripts/distribution/refresh-consumer.sh" \
            --source "$candidate" --consumer "$consumer_root" > "$FIXTURE/path-source.log" 2>&1; then
            echo 'snapshot refresh accepted a forbidden source directory' >&2; exit 1
        fi
    done
    cmp "$FIXTURE/path-manifest" "$consumer_root/.shared-tooling.snapshot"
    cmp "$FIXTURE/path-manifest" "$bad_consumer/.shared-tooling.snapshot"
    diff -r "$consumer_root/scripts" "$bad_consumer/scripts"
    [[ ! -e "$bad_consumer/.git/index" ]] || exit 1
    rm "$FIXTURE/consumer-alias" "$FIXTURE/source-alias"
done
# Relative inputs and ordinary physical aliases stay valid under hostile CDPATH.
ln -s "$consumer_root" "$FIXTURE/consumer-alias"
ln -s "$source_root" "$FIXTURE/source-alias"
(cd "$FIXTURE"; CDPATH="$FIXTURE/source" bash "$ROOT/scripts/ci/verify-shared-tooling-snapshot.sh" \
    --consumer consumer-alias) > "$FIXTURE/path-normal-verify.log"
(cd "$FIXTURE"; CDPATH="$FIXTURE/source" PATH="$FIXTURE/bin:$PATH" \
    bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source source-alias --consumer consumer-alias) > "$FIXTURE/path-normal-refresh.log"
cmp "$FIXTURE/path-manifest" "$consumer_root/.shared-tooling.snapshot"

# A resolved custom-manifest parent must obey the same directory policy.
mkdir "$consumer_root/config"$'\n'
cp "$FIXTURE/path-manifest" "$consumer_root/config"$'\n'/snapshot
ln -s $'config\n' "$consumer_root/config-alias"
if bash "$ROOT/scripts/ci/verify-shared-tooling-snapshot.sh" --consumer "$consumer_root" \
    --manifest config-alias/snapshot > "$FIXTURE/path-manifest-parent.log" 2>&1; then
    echo 'snapshot verifier accepted a forbidden resolved manifest parent' >&2; exit 1
fi
rm "$consumer_root/config-alias"
# Git output has one record terminator. Extra pathname bytes and observation
# failures cannot be normalized into the valid source root beside them.
for suffix in $'\n' $'\r'; do
    if SNAPSHOT_TEST_TOP_SUFFIX="$suffix" PATH="$FIXTURE/bin:$PATH" \
        bash "$source_root/scripts/distribution/refresh-consumer.sh" \
        --source "$source_root" --consumer "$consumer_root" > "$FIXTURE/path-git.log" 2>&1; then
        echo 'snapshot refresh trimmed a forbidden Git root observation' >&2; exit 1
    fi
done
if SNAPSHOT_TEST_TOP_FAIL=true PATH="$FIXTURE/bin:$PATH" \
    bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" --consumer "$consumer_root" > "$FIXTURE/path-git-failure.log" 2>&1; then
    echo 'snapshot refresh accepted a failed Git root observation' >&2; exit 1
fi
cmp "$FIXTURE/path-manifest" "$consumer_root/.shared-tooling.snapshot"

[[ "$(awk -F '\t' '$1 == "# version" {print $2}' "$consumer_root/.shared-tooling.snapshot")" == 0.2.8 ]] || exit 1
[[ "$(cat "$consumer_root/VERSION")" == 8.8.8 ]] || exit 1
cp "$consumer_root/.shared-tooling.snapshot" "$FIXTURE/version-manifest"
for annotation in $'# version\t01.2.3' $'# version\t0.2.8\textra' $'# version\t0.2.8\n# version\t0.2.8'; do
    sed '/^# version/d' "$FIXTURE/version-manifest" > "$consumer_root/.shared-tooling.snapshot"
    printf '%s\n' "$annotation" >> "$consumer_root/.shared-tooling.snapshot"
    cp "$consumer_root/.shared-tooling.snapshot" "$FIXTURE/invalid-version-manifest"
    if bash "$ROOT/scripts/ci/verify-shared-tooling-snapshot.sh" --consumer "$consumer_root" \
        > "$FIXTURE/invalid-version-verify.log" 2>&1; then exit 1; fi
    if PATH="$FIXTURE/bin:$PATH" bash "$source_root/scripts/distribution/refresh-consumer.sh" \
        --source "$source_root" --consumer "$consumer_root" > "$FIXTURE/invalid-version-refresh.log" 2>&1; then exit 1; fi
    cmp "$FIXTURE/invalid-version-manifest" "$consumer_root/.shared-tooling.snapshot"
done
cp "$FIXTURE/version-manifest" "$consumer_root/.shared-tooling.snapshot"
# Version annotations are display metadata. A record without one remains a
# valid v1 snapshot; a real refresh adds the version from its selected source.
sed '/^# version/d' "$FIXTURE/version-manifest" > "$consumer_root/.shared-tooling.snapshot"
bash "$ROOT/scripts/ci/verify-shared-tooling-snapshot.sh" --consumer "$consumer_root" \
    > "$FIXTURE/unrecorded-version.log"
grep -F 'version unrecorded' "$FIXTURE/unrecorded-version.log" >/dev/null
PATH="$FIXTURE/bin:$PATH" bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" --consumer "$consumer_root" >/dev/null
cmp "$FIXTURE/version-manifest" "$consumer_root/.shared-tooling.snapshot"
# Invalid or missing committed versions fail before replacing a consumer file.
for version in missing 01.2.3 $'0.2.8\n\n'; do
    if [[ "$version" == missing ]]; then rm "$revision_root/VERSION";
    else printf '%s' "$version" > "$revision_root/VERSION"; fi
    if PATH="$FIXTURE/bin:$PATH" bash "$source_root/scripts/distribution/refresh-consumer.sh" \
        --source "$source_root" --consumer "$consumer_root" > "$FIXTURE/source-version.log" 2>&1; then exit 1; fi
    cmp "$FIXTURE/version-manifest" "$consumer_root/.shared-tooling.snapshot"
    cmp "$revision_root/scripts/ci/sample.sh" "$consumer_root/scripts/ci/sample.sh"
done
printf '0.2.8' > "$revision_root/VERSION"

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
    [[ ! -e "$SNAPSHOT_HELPER_EXECUTED" ]] || exit 1
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
[[ -f "$SNAPSHOT_FALLBACK_USED" ]] || exit 1
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
[[ ! -e "$SNAPSHOT_FALLBACK_USED" ]] || exit 1

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
[[ ! -x "$consumer_root/scripts/ci/sample.sh" ]] || exit 1
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
[[ ! -e "$blocked_consumer/scripts/ci/verify-shared-tooling-snapshot.sh" ]] || exit 1

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
    [[ "$(git -C "$tracked_consumer" write-tree)" == "$index_before" ]] || exit 1
    [[ ! -e "$tracked_consumer/$tracked_manifest" ]] || exit 1
    cmp "$FIXTURE/existing-consumer-manifest" "$tracked_consumer/.shared-tooling.snapshot"
    if [[ "$state" == deleted ]]; then
        [[ ! -e "$tracked_consumer/$verifier_path" ]] || exit 1
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

# Selection expansion reads dependency declarations from committed payloads.
selection_consumer="$FIXTURE/selection"
mkdir "$selection_consumer"
git init -q "$selection_consumer"
for path in scripts/ci/run-validation-targets.sh scripts/ci/check-make-execution.sh \
    scripts/ci/run-release.sh scripts/ci/next-release-version.sh \
    scripts/ci/test-release-runner.sh scripts/ci/finalize-release-changelog.awk \
    scripts/ci/test-host-tools.sh scripts/ci/test-ic-tools.sh scripts/ci/test-tool-evidence.sh; do
    cp -p "$ROOT/$path" "$source_root/$path"
    cp -p "$ROOT/$path" "$revision_root/$path"
done
if PATH="$FIXTURE/bin:$PATH" bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" --consumer "$selection_consumer" \
    --file "$checksum_path" --file "$verifier_path" --file scripts/ci/run-validation-targets.sh \
    > "$FIXTURE/missing-companion.log" 2>&1; then exit 1; fi
[[ ! -e "$selection_consumer/scripts" && ! -e "$selection_consumer/.shared-tooling.snapshot" ]] || exit 1
PATH="$FIXTURE/bin:$PATH" bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" --consumer "$selection_consumer" --manifest config/selection \
    --file "$checksum_path" --file "$verifier_path" > "$FIXTURE/selection-initial.log"
cp "$selection_consumer/config/selection" "$FIXTURE/selection-before"
if PATH="$FIXTURE/bin:$PATH" bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" --consumer "$selection_consumer" --manifest config/selection \
    --add-file scripts/ci/run-validation-targets.sh > "$FIXTURE/missing-addition.log" 2>&1; then exit 1; fi
cmp "$FIXTURE/selection-before" "$selection_consumer/config/selection"
[[ ! -e "$selection_consumer/scripts/ci/run-validation-targets.sh" ]] || exit 1
printf 'unrelated input\n' > "$selection_consumer/local.txt"
for attempt in first retry; do
    PATH="$FIXTURE/bin:$PATH" bash "$source_root/scripts/distribution/refresh-consumer.sh" \
        --source "$source_root" --consumer "$selection_consumer" --manifest config/selection \
        --add-file scripts/ci/run-validation-targets.sh --add-file scripts/ci/check-make-execution.sh \
        > "$FIXTURE/selection-$attempt.log"
    if [[ "$attempt" == first ]]; then cp "$selection_consumer/config/selection" "$FIXTURE/selection-after"; fi
done
cmp "$FIXTURE/selection-after" "$selection_consumer/config/selection"
[[ "$(awk '$1 == "file" { n++ } END { print n }' "$selection_consumer/config/selection")" == 4 ]] || exit 1
[[ "$(cat "$selection_consumer/local.txt")" == 'unrelated input' ]] || exit 1
bash "$selection_consumer/scripts/ci/verify-shared-tooling-snapshot.sh" \
    --consumer "$selection_consumer" --manifest config/selection >/dev/null
printf 'check:\n\t@echo consumer-check-reached\n' > "$selection_consumer/Makefile"
(cd "$selection_consumer" && bash scripts/ci/run-validation-targets.sh check) > "$FIXTURE/selection-run.log"
grep -F 'consumer-check-reached' "$FIXTURE/selection-run.log" >/dev/null
PATH="$FIXTURE/bin:$PATH" bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" --consumer "$selection_consumer" --manifest config/selection \
    > "$FIXTURE/selection-refresh.log"
cmp "$FIXTURE/selection-after" "$selection_consumer/config/selection"
# Direct-only runner selection remains complete without the optional PR helper.
PATH="$FIXTURE/bin:$PATH" bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" --consumer "$selection_consumer" --manifest config/selection \
    --add-file scripts/ci/run-release.sh --add-file scripts/ci/next-release-version.sh \
    > "$FIXTURE/selection-direct.log"
[[ ! -e "$selection_consumer/scripts/ci/release-pr.sh" ]] || exit 1
# The consumer simulation has a complete explicit selection without owner-only
# native tracking fixtures. Missing changelog support still refuses atomically.
cp "$selection_consumer/config/selection" "$FIXTURE/selection-before-simulation"
if PATH="$FIXTURE/bin:$PATH" bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" --consumer "$selection_consumer" --manifest config/selection \
    --add-file scripts/ci/test-release-runner.sh > "$FIXTURE/missing-simulation-companion.log" 2>&1; then exit 1; fi
grep -F 'requires selected companion: scripts/ci/finalize-release-changelog.awk' "$FIXTURE/missing-simulation-companion.log" >/dev/null
cmp "$FIXTURE/selection-before-simulation" "$selection_consumer/config/selection"
[[ ! -e "$selection_consumer/scripts/ci/test-release-runner.sh" ]] || exit 1
PATH="$FIXTURE/bin:$PATH" bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" --consumer "$selection_consumer" --manifest config/selection \
    --add-file scripts/ci/test-release-runner.sh --add-file scripts/ci/finalize-release-changelog.awk \
    > "$FIXTURE/selection-simulation.log"
[[ ! -e "$selection_consumer/scripts/ci/test-release-tracking.sh" ]] || exit 1
bash "$selection_consumer/scripts/ci/verify-shared-tooling-snapshot.sh" \
    --consumer "$selection_consumer" --manifest config/selection >/dev/null
# Real installer fixture declarations must prevent the previously successful
# incomplete export. Check both initial export and addition to an existing set.
cp "$selection_consumer/config/selection" "$FIXTURE/selection-before-evidence"
while read -r entry missing; do
    mkdir -p "$source_root/${entry%/*}" "$revision_root/${entry%/*}"
    cp -p "$ROOT/$entry" "$source_root/$entry"
    cp -p "$ROOT/$entry" "$revision_root/$entry"
    label="${entry##*/}"
    for mode in initial addition; do
        selected_consumer="$selection_consumer"
        # The release runner is already selected in this addition fixture.
        # A present companion is not an incomplete export; initial still tests it.
        if [[ "$mode" == addition && -f "$selected_consumer/$missing" ]]; then continue; fi
        selection_args=(--manifest config/selection --add-file "$entry")
        if [[ "$mode" == initial ]]; then
            selected_consumer="$FIXTURE/incomplete-$label"
            git init -q "$selected_consumer"
            selection_args=(--file "$checksum_path" --file "$verifier_path" --file "$entry")
        fi
        if PATH="$FIXTURE/bin:$PATH" bash "$source_root/scripts/distribution/refresh-consumer.sh" \
            --source "$source_root" --consumer "$selected_consumer" "${selection_args[@]}" \
            > "$FIXTURE/missing-$label-$mode.log" 2>&1; then
            echo "snapshot distribution test failed: exported $entry without $missing ($mode)" >&2
            exit 1
        fi
        grep -F "requires selected companion: $missing" "$FIXTURE/missing-$label-$mode.log" >/dev/null
        if [[ "$mode" == initial ]]; then
            [[ ! -e "$selected_consumer/scripts" && ! -e "$selected_consumer/.shared-tooling.snapshot" ]] || exit 1
        else
            cmp "$FIXTURE/selection-before-evidence" "$selected_consumer/config/selection"
            [[ ! -e "$selected_consumer/$entry" ]] || exit 1
            bash "$ROOT/$verifier_path" --consumer "$selected_consumer" --manifest config/selection >/dev/null
        fi
    done
done <<'COMPANIONS'
scripts/ci/test-host-tools.sh scripts/ci/test-tool-evidence.sh
scripts/ci/test-ic-tools.sh scripts/ci/test-tool-evidence.sh
scripts/ci/test-tool-evidence.sh scripts/ci/select-tool-evidence.sh
scripts/ci/test-cargo-metadata.sh scripts/ci/read-cargo-workspace-version.sh
scripts/ci/test-cloc.sh scripts/dev/cloc.sh
scripts/ci/test-cloc-tooling.sh scripts/dev/cloc-tooling.pl
scripts/ci/test-crates-io-version.sh scripts/ci/check-crates-io-version.sh
scripts/ci/test-dependency-pins.sh scripts/ci/check-dependency-pins.sh
scripts/ci/test-evidence-archive.sh scripts/ci/archive-evidence.sh
scripts/ci/test-evidence-checksums.sh scripts/ci/verify-evidence-checksums.sh
scripts/ci/test-format-tools.sh scripts/ci/check-format-tools.sh
scripts/ci/test-gh-ci.sh scripts/dev/gh-ci.sh
scripts/ci/test-release-pr.sh scripts/ci/release-pr.sh
scripts/ci/test-runner-disk-space.sh scripts/ci/check-runner-disk-space.sh
scripts/ci/test-rust-tools.sh scripts/dev/install-rust-tools.sh
scripts/ci/test-rustsec-db.sh scripts/ci/prepare-rustsec-db.sh
scripts/ci/test-tool-commands.sh make/tools.mk
make/release.mk scripts/ci/run-release.sh
make/rust-format.mk make/tools.mk
make/execution.mk scripts/ci/check-make-execution.sh
make/tools.mk make/execution.mk
scripts/ci/test-make-format.sh make/tools.mk
scripts/dev/cloc-siblings.sh scripts/dev/cloc.sh
COMPANIONS

# Complete focused selections must remain independently runnable, without
# owner-only integration fixtures or unrelated production tools.
focused_consumer="$FIXTURE/focused-fixtures"
git init -q "$focused_consumer"
selection_args=(--file "$checksum_path" --file "$verifier_path")
without_reporter=("${selection_args[@]}")
for path in scripts/ci/test-format-tools.sh scripts/ci/check-format-tools.sh \
    scripts/ci/test-rust-tools.sh scripts/dev/install-rust-tools.sh \
    scripts/ci/test-make-format.sh make/tools.mk make/rust-format.mk make/execution.mk scripts/ci/check-make-execution.sh scripts/ci/run-formatting.sh; do
    mkdir -p "$source_root/${path%/*}" "$revision_root/${path%/*}"
    cp -p "$ROOT/$path" "$source_root/$path"
    cp -p "$ROOT/$path" "$revision_root/$path"
    selection_args+=(--file "$path")
    if [[ "$path" != scripts/ci/run-formatting.sh ]]; then without_reporter+=(--file "$path"); fi
done
missing_reporter="$FIXTURE/missing-format-reporter"
git init -q "$missing_reporter"
if PATH="$FIXTURE/bin:$PATH" bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" --consumer "$missing_reporter" "${without_reporter[@]}" \
    > "$FIXTURE/missing-format-reporter.log" 2>&1; then exit 1; fi
grep -F 'requires selected companion: scripts/ci/run-formatting.sh' "$FIXTURE/missing-format-reporter.log" >/dev/null
[[ ! -e "$missing_reporter/.shared-tooling.snapshot" && ! -e "$missing_reporter/scripts" ]] || exit 1
PATH="$FIXTURE/bin:$PATH" bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" --consumer "$focused_consumer" "${selection_args[@]}" \
    > "$FIXTURE/focused-fixtures-export.log"
for entry in test-format-tools test-rust-tools test-make-format; do
    bash "$focused_consumer/scripts/ci/$entry.sh" > "$FIXTURE/exported-$entry.log" 2>&1
done
bash "$ROOT/$verifier_path" --consumer "$focused_consumer" >/dev/null
# CI installer fixtures require every wrapper, even when the consumer only
# uses a subset in production. Omit each dependency from an otherwise complete
# selection so no earlier missing edge can hide a missing declaration.
installer_paths=(scripts/ci/test-installers.sh scripts/ci/install-ci-tool.sh)
for tool in actionlint gitleaks shellcheck yq sccache; do
    installer_paths+=("scripts/ci/install-$tool.sh")
done
for path in "${installer_paths[@]}"; do
    cp -p "$ROOT/$path" "$source_root/$path"
    cp -p "$ROOT/$path" "$revision_root/$path"
done
for tool in actionlint gitleaks shellcheck yq sccache; do
    missing="scripts/ci/install-$tool.sh"
    for mode in initial addition; do
        selected_consumer="$selection_consumer"
        selection_args=(--manifest config/selection)
        option=--add-file
        if [[ "$mode" == initial ]]; then
            selected_consumer="$FIXTURE/incomplete-ci-$tool"
            git init -q "$selected_consumer"
            selection_args=(--file "$checksum_path" --file "$verifier_path")
            option=--file
        fi
        for path in "${installer_paths[@]}"; do
            [[ "$path" == "$missing" ]] || selection_args+=("$option" "$path")
        done
        if PATH="$FIXTURE/bin:$PATH" bash "$source_root/scripts/distribution/refresh-consumer.sh" \
            --source "$source_root" --consumer "$selected_consumer" "${selection_args[@]}" \
            > "$FIXTURE/missing-ci-$tool-$mode.log" 2>&1; then
            echo "snapshot distribution test failed: CI fixture exported without $missing ($mode)" >&2
            exit 1
        fi
        grep -F "requires selected companion: $missing" "$FIXTURE/missing-ci-$tool-$mode.log" >/dev/null
        if [[ "$mode" == initial ]]; then
            [[ ! -e "$selected_consumer/scripts" && ! -e "$selected_consumer/.shared-tooling.snapshot" ]] || exit 1
        else
            cmp "$FIXTURE/selection-before-evidence" "$selected_consumer/config/selection"
            bash "$ROOT/$verifier_path" --consumer "$selected_consumer" --manifest config/selection >/dev/null
            [[ "$(cat "$selected_consumer/local.txt")" == 'unrelated input' ]] || exit 1
            for path in "${installer_paths[@]}"; do [[ ! -e "$selected_consumer/$path" ]] || exit 1; done
        fi
    done
done
selection_args=(--manifest config/selection)
for path in "${installer_paths[@]}"; do selection_args+=(--add-file "$path"); done
PATH="$FIXTURE/bin:$PATH" bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" --consumer "$selection_consumer" "${selection_args[@]}" \
    > "$FIXTURE/complete-ci-installers.log"
bash "$ROOT/$verifier_path" --consumer "$selection_consumer" --manifest config/selection >/dev/null
bash "$selection_consumer/scripts/ci/test-installers.sh" > "$FIXTURE/exported-ci-installers.log" 2>&1
cp "$selection_consumer/config/selection" "$FIXTURE/selection-before-conflict"
printf 'consumer-owned file\n' > "$selection_consumer/scripts/ci/sample.sh"
if PATH="$FIXTURE/bin:$PATH" bash "$source_root/scripts/distribution/refresh-consumer.sh" \
    --source "$source_root" --consumer "$selection_consumer" --manifest config/selection \
    --add-file scripts/ci/sample.sh > "$FIXTURE/addition-conflict.log" 2>&1; then exit 1; fi
cmp "$FIXTURE/selection-before-conflict" "$selection_consumer/config/selection"
[[ "$(cat "$selection_consumer/scripts/ci/sample.sh")" == 'consumer-owned file' ]] || exit 1

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

# Advance a real, unchanged uncommitted export without touching its index.
# Only these isolated source fixtures create synthetic commits.
advance_source="$FIXTURE/advance-source"
advance_seed="$FIXTURE/advance-seed"
mkdir -p "$advance_source/scripts/ci" "$advance_seed" "$FIXTURE/advance-bin"
git init -q "$advance_source"
git -C "$advance_source" config user.name 'Snapshot fixture'
git -C "$advance_source" config user.email 'fixture@example.invalid'
git -C "$advance_source" config commit.gpgsign false
git -C "$advance_source" config core.hooksPath /dev/null
git -C "$advance_source" remote add origin https://example.invalid/shared-tooling
for path in "$checksum_path" "$verifier_path"; do cp -p "$ROOT/$path" "$advance_source/$path"; done
printf '#!/usr/bin/env bash\nprintf "old snapshot\\n"\n' > "$advance_source/scripts/ci/sample.sh"
printf '0.2.8\n' > "$advance_source/VERSION"
chmod +x "$advance_source/scripts/ci/sample.sh"
git -C "$advance_source" add scripts VERSION
git -C "$advance_source" commit -qm 'Synthetic previous snapshot'
git init -q "$advance_seed"
bash "$ROOT/scripts/distribution/refresh-consumer.sh" --source "$advance_source" --consumer "$advance_seed" \
    --file "$checksum_path" --file "$verifier_path" --file scripts/ci/sample.sh > "$FIXTURE/advance-initial.log"
printf 'unrelated staged input\n' > "$advance_seed/unrelated"
git -C "$advance_seed" add unrelated
printf '#!/usr/bin/env bash\nprintf "new snapshot\\n"\n' > "$advance_source/scripts/ci/sample.sh"
printf '0.2.9\n' > "$advance_source/VERSION"
chmod -x "$advance_source/scripts/ci/sample.sh"
git -C "$advance_source" add scripts/ci/sample.sh VERSION
git -C "$advance_source" commit -qm 'Synthetic next snapshot'
export SNAPSHOT_ADVANCE_SOURCE="$advance_source"
cat > "$FIXTURE/advance-bin/git" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$1" == -C && "$2" == "$SNAPSHOT_ADVANCE_SOURCE" && "$3" == cat-file && "$4" == blob && "$5" == *:scripts/ci/* &&
    ! -e "$SNAPSHOT_ADVANCE_CONSUMER/mutated" ]]; then
    consumer="$SNAPSHOT_ADVANCE_CONSUMER"
    case "$SNAPSHOT_ADVANCE_CASE" in
        manifest-race) printf '# concurrent manifest edit\n' >> "$consumer/.shared-tooling.snapshot" ;;
        index-race) "$REAL_GIT" -C "$consumer" update-index --add --cacheinfo "100755,$SNAPSHOT_CONFLICT_BLOB,scripts/ci/sample.sh" ;;
        manifest-index-race) "$REAL_GIT" -C "$consumer" update-index --add --cacheinfo "100644,$SNAPSHOT_CONFLICT_BLOB,.shared-tooling.snapshot" ;;
        path-race)
            cp -p "$consumer/scripts/ci/sample.sh" "$consumer/replacement"
            mv "$consumer/replacement" "$consumer/scripts/ci/sample.sh" ;;
        parent-race) mv "$consumer/scripts" "$consumer/kept-scripts"; ln -s kept-scripts "$consumer/scripts" ;;
        *) exec "$REAL_GIT" "$@" ;;
    esac
    touch "$consumer/mutated"
    "$REAL_GIT" -C "$consumer" write-tree > "$consumer/expected-index"
    cp -p "$consumer/.shared-tooling.snapshot" "$consumer/expected-manifest"
fi
exec "$REAL_GIT" "$@"
SCRIPT
chmod +x "$FIXTURE/advance-bin/git"
for state in unchanged partial edited mode staged symlink forged unavailable manifest-race index-race manifest-index-race path-race parent-race; do
    consumer="$FIXTURE/advance-$state"
    cp -Rp "$advance_seed" "$consumer"
    conflict_blob="$(printf 'staged conflict\n' | git -C "$consumer" hash-object -w --stdin)"
    case "$state" in
        partial) cp -p "$advance_source/scripts/ci/sample.sh" "$consumer/scripts/ci/sample.sh" ;;
        edited) printf '# local edit\n' >> "$consumer/scripts/ci/sample.sh" ;;
        mode) chmod -x "$consumer/scripts/ci/sample.sh" ;;
        staged) git -C "$consumer" update-index --add --cacheinfo "100755,$conflict_blob,scripts/ci/sample.sh" ;;
        symlink) mv "$consumer/scripts/ci/sample.sh" "$consumer/kept-sample"; ln -s ../../kept-sample "$consumer/scripts/ci/sample.sh" ;;
        forged)
            printf '# local edit\n' >> "$consumer/scripts/ci/sample.sh"
            digest="$(bash "$ROOT/$checksum_path" --print sha256 "$consumer/scripts/ci/sample.sh")"
            awk -F '\t' -v OFS='\t' -v digest="$digest" '$1 == "file" && $4 == "scripts/ci/sample.sh" {$2=digest} {print}' \
                "$consumer/.shared-tooling.snapshot" > "$consumer/forged"
            mv "$consumer/forged" "$consumer/.shared-tooling.snapshot" ;;
        unavailable)
            awk -F '\t' -v OFS='\t' '$1 == "revision" {$2="1111111111111111111111111111111111111111"} {print}' \
                "$consumer/.shared-tooling.snapshot" > "$consumer/unavailable"
            mv "$consumer/unavailable" "$consumer/.shared-tooling.snapshot" ;;
    esac
    cp -p "$consumer/scripts/ci/sample.sh" "$consumer/expected-sample"
    cp -p "$consumer/.shared-tooling.snapshot" "$consumer/expected-manifest"
    git -C "$consumer" write-tree > "$consumer/expected-index"
    status=0
    SNAPSHOT_ADVANCE_CASE="$state" SNAPSHOT_ADVANCE_CONSUMER="$consumer" SNAPSHOT_CONFLICT_BLOB="$conflict_blob" \
        PATH="$FIXTURE/advance-bin:$PATH" bash "$ROOT/scripts/distribution/refresh-consumer.sh" \
        --source "$advance_source" --consumer "$consumer" > "$FIXTURE/advance-$state.log" 2>&1 || status=$?
    if [[ "$state" == unchanged || "$state" == partial ]]; then
        [[ "$status" == 0 ]] || exit 1
        [[ "$(awk -F '\t' '$1 == "# version" {print $2}' "$consumer/.shared-tooling.snapshot")" == 0.2.9 ]] || exit 1
        cmp "$advance_source/scripts/ci/sample.sh" "$consumer/scripts/ci/sample.sh"
        [[ ! -x "$consumer/scripts/ci/sample.sh" ]] || exit 1
        bash "$ROOT/$verifier_path" --consumer "$consumer" >/dev/null
    else
        [[ "$status" != 0 ]] || exit 1
        cmp "$consumer/expected-sample" "$consumer/scripts/ci/sample.sh"
        if [[ -x "$consumer/expected-sample" ]]; then [[ -x "$consumer/scripts/ci/sample.sh" ]] || exit 1;
        else [[ ! -x "$consumer/scripts/ci/sample.sh" ]] || exit 1; fi
        cmp "$consumer/expected-manifest" "$consumer/.shared-tooling.snapshot"
    fi
    [[ "$(git -C "$consumer" write-tree)" == "$(cat "$consumer/expected-index")" ]] || exit 1
    [[ "$(cat "$consumer/unrelated")" == 'unrelated staged input' ]] || exit 1
    if git -C "$consumer" rev-parse --verify HEAD >/dev/null 2>&1; then exit 1; fi
    cmp "$advance_seed/$checksum_path" "$consumer/$checksum_path"
    cmp "$advance_seed/$verifier_path" "$consumer/$verifier_path"
done

echo "snapshot distribution test passed"
fixture_complete=true
