#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VERSION_READER="$ROOT/scripts/ci/read-workspace-version.sh"
ELIGIBILITY_ONLY=0

fail() {
    echo "fast patch gate failed: $1" >&2
    exit 1
}

validate_compatible_lock_patch() {
    local line old_minor new_minor
    local -a old_versions=()
    local -a new_versions=()

    while IFS= read -r line; do
        case "$line" in
            diff\ --git* | index\ * | ---\ * | +++\ * | @@\ *) ;;
            -version\ =\ \"*\" | +version\ =\ \"*\" | \
                -checksum\ =\ \"*\" | +checksum\ =\ \"*\") ;;
            -* | +*)
                fail "Cargo.lock fast patches may change only compatible package versions and checksums"
                ;;
        esac
        case "$line" in
            -version\ =\ \"*\") old_versions+=("${line#-version = \"}") ;;
            +version\ =\ \"*\") new_versions+=("${line#+version = \"}") ;;
        esac
    done < <(git diff --unified=0 "$base_tag"..HEAD -- Cargo.lock)

    [ "${#old_versions[@]}" -eq "${#new_versions[@]}" ] ||
        fail "Cargo.lock package additions or removals require the complete release gate"
    [ "${#old_versions[@]}" -gt 0 ] || fail "Cargo.lock changed without a package version change"
    for ((index = 0; index < ${#old_versions[@]}; index += 1)); do
        old_versions[index]="${old_versions[index]%\"}"
        new_versions[index]="${new_versions[index]%\"}"
        old_minor="${old_versions[index]%.*}"
        new_minor="${new_versions[index]%.*}"
        [ "$old_minor" = "$new_minor" ] ||
            fail "Cargo.lock change ${old_versions[index]} -> ${new_versions[index]} is not patch-compatible"
        [ "${old_versions[index]}" != "${new_versions[index]}" ] ||
            fail "Cargo.lock version did not advance"
    done
}

case "${1:-}" in
    "") ;;
    --eligibility-only) ELIGIBILITY_ONLY=1 ;;
    *) fail "usage: $0 [--eligibility-only]" ;;
esac

cd "$ROOT"
[ -z "$(git status --porcelain)" ] || fail "source candidate is dirty"

workspace_version="$(bash "$VERSION_READER")" ||
    fail "workspace version is unavailable"
base_tag="v$workspace_version"
tag_type="$(git cat-file -t "refs/tags/$base_tag" 2>/dev/null)" ||
    fail "published baseline tag $base_tag is missing"
[ "$tag_type" = "tag" ] || fail "$base_tag is not an annotated release tag"
base_commit="$(git rev-list -n 1 "$base_tag")" ||
    fail "published baseline commit is unavailable"
git merge-base --is-ancestor "$base_commit" HEAD ||
    fail "HEAD does not descend from published baseline $base_tag"

tag_status="$(git show "$base_tag:docs/status/current.md")" ||
    fail "$base_tag does not retain its validation receipt"
mapfile -t validation_receipts < <(
    rg '^<!-- canic-release-validation: version=[0-9]+\.[0-9]+\.[0-9]+ source=[0-9a-f]{40} date=[0-9]{4}-[0-9]{2}-[0-9]{2}( gate=(complete|fast))? -->$' \
        <<<"$tag_status"
)
[ "${#validation_receipts[@]}" -eq 1 ] ||
    fail "$base_tag must retain exactly one structured validation receipt"
receipt_version="$(sed -E 's/^.*version=([^ ]+).*$/\1/' <<<"${validation_receipts[0]}")"
[ "$receipt_version" = "$workspace_version" ] ||
    fail "$base_tag validation receipt names $receipt_version instead of $workspace_version"
receipt_source="$(sed -E 's/^.*source=([0-9a-f]{40}).*$/\1/' <<<"${validation_receipts[0]}")"
git cat-file -e "$receipt_source^{commit}" 2>/dev/null ||
    fail "$base_tag validation receipt source is unavailable"
git merge-base --is-ancestor "$receipt_source" "$base_commit" ||
    fail "$base_tag validation receipt source does not precede its release"
validation_basis_tag="$base_tag"
if [[ "${validation_receipts[0]}" == *" gate=fast -->" ]]; then
    validation_basis_tag=""
    while IFS= read -r candidate_tag; do
        [ "$candidate_tag" != "$base_tag" ] || continue
        candidate_version="${candidate_tag#v}"
        [[ "$candidate_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || continue
        [ "$(git cat-file -t "refs/tags/$candidate_tag" 2>/dev/null || true)" = "tag" ] || continue
        candidate_pattern="${candidate_version//./\\.}"
        candidate_status="$(git show "$candidate_tag:docs/status/current.md" 2>/dev/null || true)"
        mapfile -t complete_receipts < <(
            rg "^<!-- canic-release-validation: version=$candidate_pattern source=[0-9a-f]{40} date=[0-9]{4}-[0-9]{2}-[0-9]{2} -->$|^<!-- canic-release-validation: version=$candidate_pattern source=[0-9a-f]{40} date=[0-9]{4}-[0-9]{2}-[0-9]{2} gate=complete -->$" \
                <<<"$candidate_status"
        )
        if [ "${#complete_receipts[@]}" -eq 1 ]; then
            candidate_source="$(sed -E 's/^.*source=([0-9a-f]{40}).*$/\1/' <<<"${complete_receipts[0]}")"
            candidate_commit="$(git rev-list -n 1 "$candidate_tag")"
            git cat-file -e "$candidate_source^{commit}" 2>/dev/null || continue
            git merge-base --is-ancestor "$candidate_source" "$candidate_commit" || continue
            validation_basis_tag="$candidate_tag"
            break
        fi
    done < <(git tag --merged "$base_commit" --sort=-version:refname 'v*')
    [ -n "$validation_basis_tag" ] ||
        fail "$base_tag has a fast receipt but no complete validated release ancestor"
fi

mapfile -t changed_paths < <(git diff --name-only "$base_tag"..HEAD)
[ "${#changed_paths[@]}" -gt 0 ] || fail "no patch changes exist after $base_tag"

lock_changed=0
release_tooling_changed=0
changelog_changed=0
for changed_path in "${changed_paths[@]}"; do
    case "$changed_path" in
        AGENTS.md | docs/*)
            ;;
        CHANGELOG.md)
            changelog_changed=1
            ;;
        Cargo.lock)
            lock_changed=1
            ;;
        Makefile | \
            scripts/ci/bump-version.sh | \
            scripts/ci/check-current-document-semantics.sh | \
            scripts/ci/check-fast-patch-eligibility.sh | \
            scripts/ci/check-release-candidate.sh | \
            scripts/ci/check-release-integrity-contract.sh | \
            scripts/ci/confirm-version-bump.sh | \
            crates/canic/tests/release_flow_guard.rs)
            release_tooling_changed=1
            ;;
        *)
            fail "runtime, build, package, protocol, fixture, or unrelated path changed: $changed_path"
            ;;
    esac
done

git diff --check "$base_tag"..HEAD || fail "diff hygiene failed"
[ "$lock_changed" -eq 0 ] || validate_compatible_lock_patch
echo "fast patch eligibility passed against $base_tag using complete basis $validation_basis_tag (${#changed_paths[@]} changed paths)"

[ "$ELIGIBILITY_ONLY" -eq 0 ] || exit 0

bash scripts/ci/check-current-document-semantics.sh
bash scripts/ci/check-release-validation-matrix.sh

if [ "$release_tooling_changed" -eq 1 ]; then
    cargo fmt --all -- --check
    make --no-print-directory shellcheck
    bash scripts/ci/check-release-integrity-contract.sh
    cargo test --locked -p canic --test release_flow_guard -- --nocapture
fi

if [ "$changelog_changed" -eq 1 ]; then
    cargo test --locked -p canic --test changelog_governance -- --nocapture
fi

if [ "$lock_changed" -eq 1 ]; then
    bash scripts/ci/check-dependency-risk-inventory.sh
    cargo metadata --locked --offline --format-version 1 >/dev/null
    cargo check --locked --workspace --all-targets
fi

echo "FAST PATCH VALIDATION PASSED: targeted non-runtime gates succeeded; PocketIC was not run"
