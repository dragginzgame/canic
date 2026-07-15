#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT_DIR"

mapfile -t STAGED_FILES < <(git diff --cached --name-only --diff-filter=ACMRD)

if [[ ${#STAGED_FILES[@]} -eq 0 ]]; then
  echo "No staged release files; run make release-stage first." >&2
  exit 1
fi

mapfile -t DELETED_FILES < <(git diff --cached --name-only --diff-filter=D)

if [[ ${#DELETED_FILES[@]} -ne 0 ]]; then
  echo "Release commit index contains staged deletions:" >&2
  printf '  %s\n' "${DELETED_FILES[@]}" >&2
  echo "Commit file removals separately before running make release-commit." >&2
  exit 1
fi

is_release_file() {
  case "$1" in
    Cargo.toml | \
      Cargo.lock | \
      scripts/dev/install_dev.sh | \
      scripts/ci/sync-release-surface-version.sh)
      return 0
      ;;
    */Cargo.toml)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

INVALID_FILES=()
for file in "${STAGED_FILES[@]}"; do
  if ! is_release_file "$file"; then
    INVALID_FILES+=("$file")
  fi
done

if [[ ${#INVALID_FILES[@]} -ne 0 ]]; then
  echo "Release commit index contains non-release files:" >&2
  printf '  %s\n' "${INVALID_FILES[@]}" >&2
  echo "Commit code/docs changes separately before running make release-commit." >&2
  exit 1
fi

mapfile -t DIRTY_FILES < <(git diff --name-only)
PARTIAL_FILES=()
for staged in "${STAGED_FILES[@]}"; do
  for dirty in "${DIRTY_FILES[@]}"; do
    if [[ "$staged" == "$dirty" ]]; then
      PARTIAL_FILES+=("$staged")
    fi
  done
done

if [[ ${#PARTIAL_FILES[@]} -ne 0 ]]; then
  echo "Release files are staged with additional unstaged changes:" >&2
  printf '  %s\n' "${PARTIAL_FILES[@]}" >&2
  echo "Stage the final release file contents or split the work before release-commit." >&2
  exit 1
fi

echo "✅ Release index contains only complete release files"
