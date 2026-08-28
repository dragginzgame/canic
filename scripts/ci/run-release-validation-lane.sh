#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VALIDATION_KIND="${1:-}"
BUMP_TYPE="${2:-}"

case "$VALIDATION_KIND/$BUMP_TYPE" in
    complete/patch | complete/minor | complete/major | fast/patch) ;;
    *)
        echo "usage: $0 <complete|fast> <patch|minor|major>" >&2
        exit 2
        ;;
esac

cd "$ROOT"
make --no-print-directory ensure-clean
bash scripts/ci/check-release-draft-ready.sh "$BUMP_TYPE"
validated_head="$(git rev-parse HEAD)"
receipt_dir="${CANIC_RELEASE_RECEIPT_DIR:-$ROOT/target/release-validation}"
receipt="$receipt_dir/$VALIDATION_KIND-$validated_head.receipt"
expected_receipt="schema=1
source=$validated_head
kind=$VALIDATION_KIND"
receipt_valid=0

if [[ "$VALIDATION_KIND" = complete && -f "$receipt" && "$(cat "$receipt")" = "$expected_receipt" ]]; then
    receipt_valid=1
    echo "✅ Reusing complete validation receipt for $validated_head"
else
    case "$VALIDATION_KIND" in
        complete)
            make --no-print-directory validate
            ;;
        fast)
            bash scripts/ci/check-fast-patch-eligibility.sh
            ;;
    esac
fi

make --no-print-directory ensure-clean
current_head="$(git rev-parse HEAD)"
if [[ "$validated_head" != "$current_head" ]]; then
    echo "validated source changed during the release gate" >&2
    exit 1
fi
if [[ "$VALIDATION_KIND" = complete && "$receipt_valid" -eq 0 ]]; then
    mkdir -p "$receipt_dir"
    temporary_receipt="$receipt.tmp.$$"
    printf '%s\n' "$expected_receipt" >"$temporary_receipt"
    mv "$temporary_receipt" "$receipt"
    echo "✅ Retained complete validation receipt for $validated_head"
fi

CANIC_RELEASE_VALIDATED=1 \
CANIC_RELEASE_VALIDATED_HEAD="$validated_head" \
CANIC_RELEASE_VALIDATION_KIND="$VALIDATION_KIND" \
    scripts/ci/bump-version.sh "$BUMP_TYPE"
