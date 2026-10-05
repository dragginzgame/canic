#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
    preflight)
        [[ "$(bash scripts/ci/read-workspace-version.sh)" == "${RELEASE_PREVIOUS:?}" ]]
        paths="$(mktemp "${TMPDIR:-/tmp}/canic-release-paths.XXXXXX")"
        trap 'rm -f "$paths"' EXIT
        git diff --name-only -z HEAD -- > "$paths"
        git ls-files --others --exclude-standard -z >> "$paths"
        while IFS= read -r -d '' path; do
            case "$path" in CHANGELOG.md|docs/changelog/*.md) ;;
                *) printf 'uncommitted non-release path: %q\n' "$path" >&2; exit 1 ;;
            esac
        done < "$paths"
        cargo set-version --help >/dev/null
        cargo fetch --locked --offline
        bash scripts/ci/check-release-draft-ready.sh "${RELEASE_KIND:?}"
        ;;
    files)
        printf '%s\0' Cargo.toml Cargo.lock scripts/dev/install_dev.sh release-validation.json CHANGELOG.md "docs/changelog/${RELEASE_VERSION%.*}.md"
        git ls-files -z -- ':(glob)crates/**/Cargo.toml' ':(glob)testing/**/Cargo.toml'
        ;;
    tagged)
        bash scripts/ci/check-release-candidate.sh
        [[ "$(git cat-file -t "refs/tags/v${RELEASE_VERSION:?}")" == tag ]]
        [[ "$(git rev-parse "refs/tags/v$RELEASE_VERSION^{commit}")" == "$(git rev-parse HEAD)" ]]
        ;;
    *) echo 'usage: adapter.sh preflight|files|tagged' >&2; exit 2 ;;
esac
