#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
    preflight)
        [[ "$(bash scripts/ci/read-workspace-version.sh)" == "${RELEASE_PREVIOUS:?}" ]]
        paths="$(mktemp "${TMPDIR:-/tmp}/canic-release-paths.XXXXXX")"
        trap 'rm -f "$paths"' EXIT
        git diff --name-only -z -- > "$paths"
        git diff --cached --name-only -z -- >> "$paths"
        git ls-files --others --exclude-standard -z >> "$paths"
        while IFS= read -r -d '' path; do
            case "$path" in CHANGELOG.md|docs/changelog/*.md) ;;
                *) printf 'uncommitted non-release path: %q\n' "$path" >&2; exit 1 ;;
            esac
        done < "$paths"
        bash scripts/ci/check-release-draft-ready.sh "${RELEASE_KIND:?}"
        cargo set-version --help >/dev/null
        cargo fetch --locked
        ;;
    files)
        printf '%s\0' Cargo.toml Cargo.lock scripts/dev/install_dev.sh release-validation.json CHANGELOG.md "docs/changelog/${RELEASE_VERSION%.*}.md"
        git ls-files -z -- ':(glob)crates/**/Cargo.toml' ':(glob)testing/**/Cargo.toml'
        ;;
    committed)
        bash scripts/ci/check-release-candidate.sh --commit "${RELEASE_COMMIT:?}"
        ;;
    tagged)
        bash scripts/ci/check-release-candidate.sh --commit "${RELEASE_COMMIT:?}"
        bash scripts/ci/check-release-tag.sh "$RELEASE_COMMIT" "${RELEASE_VERSION:?}"
        ;;
    *) echo 'usage: adapter.sh preflight|files|committed|tagged' >&2; exit 2 ;;
esac
