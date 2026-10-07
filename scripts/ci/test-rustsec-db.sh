#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd -P)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/rustsec-db-test.XXXXXX")"
# Local source selection uses physical paths, including macOS /var aliases.
fixture="$(cd "$fixture" && pwd -P)"
finish() {
    local status=$?
    if [[ "$status" == 0 ]]; then rm -rf "$fixture";
    else echo "RustSec preparation fixtures retained: $fixture" >&2; fi
}
trap finish EXIT
checker="$root/scripts/ci/prepare-rustsec-db.sh"
mkdir "$fixture/bin" "$fixture/local source"
export RUSTSEC_FIXTURE="$fixture"
revision=1111111111111111111111111111111111111111
export RUSTSEC_REVISION="$revision"
cat > "$fixture/bin/git" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\0' "$@" >> "$RUSTSEC_FIXTURE/arguments"
printf '%s\n' "$GIT_ALLOW_PROTOCOL" >> "$RUSTSEC_FIXTURE/protocols"
[[ "$GIT_TERMINAL_PROMPT" == 0 ]] || exit 1
if [[ "$1" == -C ]]; then
    case "$3" in
        rev-parse)
            [[ "${RUSTSEC_MODE:-pass}" != observe-fail ]] || exit 7
            if [[ "${RUSTSEC_MODE:-pass}" == malformed ]]; then echo invalid;
            elif [[ "${RUSTSEC_MODE:-pass}" == wrong-head && "$2" == */db ]]; then
                echo 2222222222222222222222222222222222222222
            else echo "$RUSTSEC_REVISION"; fi ;;
        checkout) [[ "${RUSTSEC_MODE:-pass}" != checkout-fail ]] ;;
        *) exit 99 ;;
    esac
else
    destination="${!#}"
    mkdir -p "$destination/.git/objects/info"
    echo 'clone diagnostic' >&2
    [[ "${RUSTSEC_MODE:-pass}" != clone-fail ]] || exit 8
    if [[ "${RUSTSEC_MODE:-pass}" == borrowed ]]; then
        echo /borrowed/objects > "$destination/.git/objects/info/alternates"
    fi
fi
STUB
chmod +x "$fixture/bin/git"
prepare() { PATH="$fixture/bin:$PATH" bash "$checker" "$@"; }
for mode in online local; do
    source=https://github.com/RustSec/advisory-db.git
    [[ "$mode" != local ]] || source="$fixture/local source"
    : > "$fixture/arguments"
    : > "$fixture/protocols"
    prepare "$mode" "$source" "$fixture/$mode result" > "$fixture/output"
    [[ "$(cat "$fixture/output")" == "$revision" ]] || exit 1
    [[ "$(cat "$fixture/$mode result/revision")" == "$revision" ]] || exit 1
    rg 'clone diagnostic' "$fixture/$mode result/prepare.log" >/dev/null
    protocol=https; [[ "$mode" != local ]] || protocol='file'
    [[ "$(sort -u "$fixture/protocols")" == "$protocol" ]] || exit 1
    if [[ "$mode" == local ]]; then
        printf '%s\0' -C "$source" rev-parse --verify 'HEAD^{commit}' \
            clone --local --no-hardlinks --dissociate --no-checkout --no-recurse-submodules -- "$source" "$fixture/local result/db" \
            -C "$fixture/local result/db" checkout --detach "$revision" -- \
            -C "$fixture/local result/db" rev-parse --verify 'HEAD^{commit}' > "$fixture/expected"
    else
        printf '%s\0' -c http.lowSpeedLimit=1024 -c http.lowSpeedTime=30 \
            clone --depth 1 --single-branch --no-tags --no-recurse-submodules -- "$source" "$fixture/online result/db" \
            -C "$fixture/online result/db" rev-parse --verify 'HEAD^{commit}' \
            -C "$fixture/online result/db" rev-parse --verify 'HEAD^{commit}' > "$fixture/expected"
    fi
    cmp "$fixture/expected" "$fixture/arguments"
done
for failure in observe-fail malformed wrong-head clone-fail checkout-fail borrowed; do
    destination="$fixture/$failure"
    # A consumer's audit admission must depend on preparation success.
    if RUSTSEC_MODE="$failure" prepare local "$fixture/local source" "$destination" > "$fixture/output" 2> "$fixture/error"; then
        touch "$fixture/audit-invoked"
    fi
    [[ ! -e "$fixture/audit-invoked" && ! -e "$destination/revision" && ! -s "$fixture/output" ]] || exit 1
    [[ -f "$destination/prepare.log" ]] || exit 1
    rg -F "$destination" "$fixture/error" >/dev/null
done
# An online acquisition failure is retained, with no retry/fallback.
: > "$fixture/arguments"
if RUSTSEC_MODE=clone-fail prepare online https://github.com/RustSec/advisory-db.git "$fixture/online failure" > "$fixture/output" 2> "$fixture/error"; then exit 1; fi
[[ -f "$fixture/online failure/prepare.log" && ! -e "$fixture/online failure/revision" ]] || exit 1
printf '%s\0' -c http.lowSpeedLimit=1024 -c http.lowSpeedTime=30 \
    clone --depth 1 --single-branch --no-tags --no-recurse-submodules -- \
    https://github.com/RustSec/advisory-db.git "$fixture/online failure/db" > "$fixture/expected"
cmp "$fixture/expected" "$fixture/arguments"
: > "$fixture/arguments"
for mode in invalid online local; do
    if prepare "$mode" missing "$fixture/invalid-$mode" > /dev/null 2>&1; then exit 1; fi
done
for source in http://example.com/db https://user:token@example.com/db 'https://example.com/db?token=x'; do
    if prepare online "$source" "$fixture/invalid-url" > /dev/null 2>&1; then exit 1; fi
done
mkdir "$fixture/occupied"
echo preserve > "$fixture/occupied/evidence"
ln -s "$fixture/nonexistent" "$fixture/occupied-link"
for destination in "$fixture/occupied" "$fixture/occupied-link"; do
    if prepare local "$fixture/local source" "$destination" > /dev/null 2>&1; then exit 1; fi
done
[[ ! -s "$fixture/arguments" && "$(cat "$fixture/occupied/evidence")" == preserve ]] || exit 1

# Real local Git proves dissociation even when the selected source borrows objects.
# Reuse existing history; never create a commit or contact a remote.
git init --quiet "$fixture/borrowed source"
objects="$(git -C "$root" rev-parse --git-path objects)"
case "$objects" in /*) ;; *) objects="$root/$objects" ;; esac
printf '%s\n' "$objects" > "$fixture/borrowed source/.git/objects/info/alternates"
# Explicitly construct the borrowing precondition, including shallow CI history.
shallow="$(git -C "$root" rev-parse --git-path shallow)"
case "$shallow" in /*) ;; *) shallow="$root/$shallow" ;; esac
if [[ -f "$shallow" ]]; then cp "$shallow" "$fixture/borrowed source/.git/shallow"; fi
git -C "$fixture/borrowed source" update-ref HEAD "$(git -C "$root" rev-parse HEAD)"
[[ -s "$fixture/borrowed source/.git/objects/info/alternates" ]] || exit 1
printf 'local source evidence\n' > "$fixture/borrowed source/untracked-evidence"
selected="$(git -C "$fixture/borrowed source" rev-parse HEAD)"
GIT_DIR="$fixture/wrong-git-dir" GIT_WORK_TREE="$fixture/wrong-worktree" \
    GIT_INDEX_FILE="$fixture/wrong-index" bash "$checker" local \
    "$fixture/borrowed source" "$fixture/isolated result" > "$fixture/output"
[[ "$(cat "$fixture/output")" == "$selected" ]] || exit 1
[[ ! -s "$fixture/isolated result/db/.git/objects/info/alternates" ]] || exit 1
[[ ! -e "$fixture/isolated result/db/untracked-evidence" ]] || exit 1
[[ -f "$fixture/borrowed source/untracked-evidence" && ! -e "$fixture/wrong-index" ]] || exit 1
# Pack and loose objects must have one hardlink; the database can outlive its source.
perl -MFile::Find -e 'find(sub { -f $_ && (stat($_))[3] != 1 and die "shared object: $File::Find::name\n" }, $ARGV[0])' \
    "$fixture/isolated result/db/.git/objects"
mv "$fixture/borrowed source" "$fixture/source removed"
git -C "$fixture/isolated result/db" cat-file -e "$selected^{commit}"
echo 'RustSec source selection, failure retention, network policy and local isolation tests passed'
