#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VERSION_READER="$ROOT/scripts/ci/read-workspace-version.sh"
ELIGIBILITY_ONLY=0

fail() {
    echo "fast patch gate failed: $1" >&2
    exit 1
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

receipt="$(bash "$ROOT/scripts/ci/read-release-validation.sh" "$base_tag" "$workspace_version")" ||
    fail "$base_tag has no valid structured validation receipt; use the complete release gate"
IFS=$'\t' read -r receipt_source receipt_gate <<<"$receipt"
git cat-file -e "$receipt_source^{commit}" 2>/dev/null ||
    fail "$base_tag validation receipt source is unavailable"
git merge-base --is-ancestor "$receipt_source" "$base_commit" ||
    fail "$base_tag validation receipt source does not precede its release"
validation_basis_tag="$base_tag"
if [[ "$receipt_gate" == fast ]]; then
    validation_basis_tag=""
    while IFS= read -r candidate_tag; do
        [ "$candidate_tag" != "$base_tag" ] || continue
        candidate_version="${candidate_tag#v}"
        [[ "$candidate_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || continue
        [ "$(git cat-file -t "refs/tags/$candidate_tag" 2>/dev/null || true)" = "tag" ] || continue
        candidate_receipt="$(bash "$ROOT/scripts/ci/read-release-validation.sh" "$candidate_tag" "$candidate_version" 2>/dev/null)" || continue
        IFS=$'\t' read -r candidate_source candidate_gate <<<"$candidate_receipt"
        [[ "$candidate_gate" == complete ]] || continue
        candidate_commit="$(git rev-list -n 1 "$candidate_tag")"
        git cat-file -e "$candidate_source^{commit}" 2>/dev/null || continue
        git merge-base --is-ancestor "$candidate_source" "$candidate_commit" || continue
        validation_basis_tag="$candidate_tag"
        break
    done < <(git tag --merged "$base_commit" --sort=-version:refname 'v*')
    [ -n "$validation_basis_tag" ] ||
        fail "$base_tag has a fast receipt but no complete validated release ancestor"
fi

mapfile -t changed_paths < <(git diff --name-only "$base_tag"..HEAD)
[ "${#changed_paths[@]}" -gt 0 ] || fail "no patch changes exist after $base_tag"

release_tooling_changed=0
changelog_changed=0
for changed_path in "${changed_paths[@]}"; do
    case "$changed_path" in
        AGENTS.md | docs/*)
            ;;
        CHANGELOG.md)
            changelog_changed=1
            ;;
        Makefile | \
            scripts/ci/bump-version.sh | \
            scripts/ci/check-current-document-semantics.sh | \
            scripts/ci/check-fast-patch-eligibility.sh | \
            scripts/ci/check-release-candidate.sh | \
            scripts/ci/check-release-integrity-contract.sh | \
            scripts/ci/read-release-validation.sh | \
            crates/canic/tests/release_flow_guard.rs)
            release_tooling_changed=1
            ;;
        *)
            fail "runtime, build, package, protocol, fixture, or unrelated path changed: $changed_path"
            ;;
    esac
done

git diff --check "$base_tag"..HEAD || fail "diff hygiene failed"
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


echo "FAST PATCH VALIDATION PASSED: targeted non-runtime gates succeeded; PocketIC was not run"
