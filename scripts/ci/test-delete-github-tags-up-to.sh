#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DELETE_SCRIPT="$ROOT_DIR/scripts/dev/delete-github-tags-up-to.sh"
FIXTURE_ROOT="$(mktemp -d)"

cleanup() {
    rm -rf "$FIXTURE_ROOT"
}
trap cleanup EXIT

fail() {
    echo "tag-deletion fixture failed: $1" >&2
    exit 1
}

REMOTE_REPO="$FIXTURE_ROOT/remote.git"
SOURCE_REPO="$FIXTURE_ROOT/source"
FIXTURE_SCRIPT="$SOURCE_REPO/scripts/dev/delete-github-tags-up-to.sh"

git init --bare --initial-branch=main "$REMOTE_REPO" >/dev/null
git init --initial-branch=main "$SOURCE_REPO" >/dev/null
git -C "$SOURCE_REPO" config user.name "Canic CI"
git -C "$SOURCE_REPO" config user.email "ci@example.invalid"
git -C "$SOURCE_REPO" commit --allow-empty -m "fixture" >/dev/null
git -C "$SOURCE_REPO" remote add origin "$REMOTE_REPO"

for tag in v0.59.9 v0.60.0 v0.60.1 v0.61.0; do
    git -C "$SOURCE_REPO" tag -a "$tag" -m "$tag"
done
git -C "$SOURCE_REPO" push origin main --tags >/dev/null 2>&1

mkdir -p "$(dirname "$FIXTURE_SCRIPT")"
cp "$DELETE_SCRIPT" "$FIXTURE_SCRIPT"

# A rejected remote deletion must leave the local recovery copies intact.
cat >"$REMOTE_REPO/hooks/pre-receive" <<'HOOK'
#!/usr/bin/env bash
set -euo pipefail

while read -r _old new ref; do
    if [[ "$new" == "0000000000000000000000000000000000000000" &&
          "$ref" == refs/tags/* ]]; then
        exit 1
    fi
done
HOOK
chmod +x "$REMOTE_REPO/hooks/pre-receive"
if (
    cd "$SOURCE_REPO"
    bash "$FIXTURE_SCRIPT" \
        --cutoff v0.60.0 \
        --delete-local \
        --delete-remote \
        --yes
) >/dev/null 2>&1; then
    fail "remote deletion rejection was accepted"
fi
for tag in v0.59.9 v0.60.0; do
    git -C "$SOURCE_REPO" show-ref --verify --quiet "refs/tags/$tag" ||
        fail "remote failure removed local recovery tag $tag"
done
rm "$REMOTE_REPO/hooks/pre-receive"

deletion_output="$({
    cd "$SOURCE_REPO"
    bash "$FIXTURE_SCRIPT" \
        --cutoff v0.60.0 \
        --delete-local \
        --delete-remote \
        --yes
} 2>&1)"

grep -Fq "remote origin deletion verified through v0.60.0" <<<"$deletion_output" ||
    fail "remote verification receipt is missing"
grep -Fq "local deletion verified through v0.60.0" <<<"$deletion_output" ||
    fail "local verification receipt is missing"

expected_tags=$'v0.60.1\nv0.61.0'
local_tags="$(git -C "$SOURCE_REPO" tag --list | sort -V)"
[[ "$local_tags" == "$expected_tags" ]] ||
    fail "unexpected local tags remain: $local_tags"

remote_tags="$({
    git -C "$SOURCE_REPO" ls-remote --tags --refs origin |
        awk '{ sub("^refs/tags/", "", $2); print $2 }' |
        sort -V
})"
[[ "$remote_tags" == "$expected_tags" ]] ||
    fail "unexpected remote tags remain: $remote_tags"

# A broad push from the cleaned clone must not recreate the deleted refs.
git -C "$SOURCE_REPO" push origin --tags >/dev/null 2>&1
remote_tags_after_push="$({
    git -C "$SOURCE_REPO" ls-remote --tags --refs origin |
        awk '{ sub("^refs/tags/", "", $2); print $2 }' |
        sort -V
})"
[[ "$remote_tags_after_push" == "$expected_tags" ]] ||
    fail "a broad push from the cleaned clone recreated deleted tags"

echo "tag-deletion local/remote verification fixture passed"
