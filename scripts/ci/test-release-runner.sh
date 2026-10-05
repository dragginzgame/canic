#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIXTURE_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/release-runner-test.XXXXXX")"
trap 'rm -rf "$FIXTURE_ROOT"' EXIT
export REAL_GIT
REAL_GIT="$(command -v git)"
mkdir -p "$FIXTURE_ROOT/bin"

cat > "$FIXTURE_ROOT/bin/make" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
while [[ "$1" == --no-print-directory || "$1" == -s ]]; do shift; done
target="$1"
shift
for argument in "$@"; do export "$argument"; done
if [[ "$target" == release-version ]]; then cat version; exit; fi
printf '%s\n' "$target" >> events
[[ "${FIXTURE_FAIL_TARGET:-}" != "$target" ]] || exit 7
case "$target" in
    release-preflight) [[ "$(cat version)" == "$RELEASE_PREVIOUS" ]] ;;
    release-verify) ;;
    release-prepare-version) printf '%s\n' "$RELEASE_VERSION" > version ;;
    release-prepared-check) [[ "$(cat version)" == "$RELEASE_VERSION" ]] ;;
    release-files) printf 'version\0release file.txt\0' ;;
    release-commit-check|release-committed-check|release-tagged-check|release-push-check) ;;
    *) exit 2 ;;
esac
STUB

cat > "$FIXTURE_ROOT/bin/git" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
source_sha=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
release_sha=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
tag_sha=cccccccccccccccccccccccccccccccccccccccc
case "$1" in
    check-ref-format) [[ "$2" == refs/heads/main ]] ;;
    symbolic-ref) echo main ;;
    remote) printf '%s\n' "${FIXTURE_DESTINATION:-https://example.invalid/release-fixture}" ;;
    hash-object) exec "$REAL_GIT" hash-object --stdin ;;
    rev-parse)
        case "${*: -1}" in
            --show-toplevel) pwd ;;
            release-state) echo .release-state ;;
            HEAD) cat head ;;
            HEAD^) [[ "$(cat head)" == "$release_sha" ]]; echo "$source_sha" ;;
            'HEAD^{tree}') echo dddddddddddddddddddddddddddddddddddddddd ;;
            refs/tags/*'^{commit}') [[ -f tag ]]; echo "$release_sha" ;;
            refs/tags/*) [[ -f tag ]]; echo "$tag_sha" ;;
            *) exit 2 ;;
        esac
        ;;
    diff) if [[ "$2" != --quiet ]]; then cat version; fi ;;
    ls-files) ;;
    write-tree) echo dddddddddddddddddddddddddddddddddddddddd ;;
    log) printf 'Release %s\n' "$(cat version)" ;;
    tag)
        case "$2" in
            --list) if [[ -f tag && "$(cat tag)" == "${3#v}" ]]; then echo "$3"; fi ;;
            -a)
                printf '%s\n' "${3#v}" > tag
                echo tag >> events
                if [[ "${FIXTURE_FAIL_EFFECT:-}" == tag && ! -f lost-tag ]]; then touch lost-tag; exit 9; fi
                ;;
            *) exit 2 ;;
        esac
        ;;
    cat-file) [[ -f tag ]]; echo tag ;;
    ls-remote)
        for ref in "$@"; do
            case "$ref" in
                refs/heads/main) if [[ -f remote-head ]]; then printf '%s\t%s\n' "$(cat remote-head)" "$ref"; fi ;;
                refs/tags/*) if [[ -f remote-tag ]]; then printf '%s\t%s\n' "$(cat remote-tag)" "$ref"; fi ;;
            esac
        done
        ;;
    add)
        [[ "$#" == 4 && "$2" == -- && "$3" == version && "$4" == 'release file.txt' ]]
        echo stage >> events
        ;;
    commit)
        [[ "$#" == 3 && "$2" == -m && "$3" == "Release $(cat version)" ]]
        echo "$release_sha" > head
        echo commit >> events
        if [[ "${FIXTURE_FAIL_EFFECT:-}" == commit && ! -f lost-commit ]]; then touch lost-commit; exit 9; fi
        ;;
    push)
        [[ "$#" == 6 && "$2" == --no-follow-tags && "$3" == --atomic && "$4" == origin && "$5" == HEAD:refs/heads/main && "$6" == "refs/tags/v$(cat version):refs/tags/v$(cat version)" ]]
        echo push >> events
        [[ "${FIXTURE_FAIL_EFFECT:-}" != before-push ]] || exit 9
        echo "$release_sha" > remote-head
        echo "$tag_sha" > remote-tag
        if [[ "${FIXTURE_FAIL_EFFECT:-}" == push && ! -f lost-push ]]; then touch lost-push; exit 9; fi
        ;;
    *) exit 2 ;;
esac
STUB
chmod +x "$FIXTURE_ROOT/bin/make" "$FIXTURE_ROOT/bin/git"
export PATH="$FIXTURE_ROOT/bin:$PATH"

new_fixture() {
    mkdir -p "$FIXTURE_ROOT/$1"
    cd "$FIXTURE_ROOT/$1"
    printf '0.1.0\n' > version
    printf 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n' > 'head'
    printf 'retained build artifact\n' > 'release file.txt'
}
count_event() { awk -v event="$1" '$0 == event { count++ } END { print count+0 }' events; }
expect_failure() {
    if bash "$ROOT/scripts/ci/run-release.sh" "$@" > output 2>&1; then
        echo 'release runner unexpectedly accepted a failing fixture' >&2
        exit 1
    fi
}

for kind in patch minor major; do
    new_fixture "$kind"
    case "$kind" in patch) candidate=0.1.1 ;; minor) candidate=0.2.0 ;; major) candidate=1.0.0 ;; esac
    bash "$ROOT/scripts/ci/run-release.sh" "$kind" origin main > output
    [[ "$(cat version)" == "$candidate" && "$(cat tag)" == "$candidate" ]]
    awk '/^(release-verify|release-prepare-version|stage|commit|tag|push)$/ { print }' events > observed
    printf '%s\n' release-verify release-prepare-version stage commit tag push > expected
    cmp expected observed
    [[ "$(count_event commit)" == 1 && "$(count_event push)" == 1 ]]
    [[ "$(tail -n 1 ".release-state/$candidate.plan")" == complete ]]
    [[ "$(cat 'release file.txt')" == 'retained build artifact' ]]
    [[ ! -e .release-state/lock ]]
done

for target in release-preflight release-verify release-prepare-version release-prepared-check release-commit-check release-committed-check release-tagged-check release-push-check; do
    new_fixture "$target"
    export FIXTURE_FAIL_TARGET="$target"
    expect_failure patch origin main
    [[ "$(count_event push)" == 0 && ! -e .release-state/lock ]]
    case "$target" in
        release-preflight|release-verify|release-prepare-version) [[ "$(cat version)" == 0.1.0 ]] ;;
    esac
    unset FIXTURE_FAIL_TARGET
    bash "$ROOT/scripts/ci/run-release.sh" resume 0.1.1 origin main > output
    [[ "$(count_event commit)" == 1 && "$(count_event push)" == 1 ]]
    [[ "$(cat version)" == 0.1.1 ]]
done

for effect in commit tag push before-push; do
    new_fixture "lost-$effect"
    export FIXTURE_FAIL_EFFECT="$effect"
    expect_failure patch origin main
    unset FIXTURE_FAIL_EFFECT
    bash "$ROOT/scripts/ci/run-release.sh" resume 0.1.1 origin main > output
    [[ "$(count_event release-prepare-version)" == 1 && "$(count_event commit)" == 1 ]]
    case "$effect" in before-push) [[ "$(count_event push)" == 2 ]] ;; *) [[ "$(count_event push)" == 1 ]] ;; esac
done

new_fixture destination-change
export FIXTURE_FAIL_EFFECT=before-push
expect_failure patch origin main
unset FIXTURE_FAIL_EFFECT
export FIXTURE_DESTINATION=https://example.invalid/other
expect_failure resume 0.1.1 origin main
[[ "$(count_event push)" == 1 ]]
unset FIXTURE_DESTINATION

new_fixture concurrent
mkdir -p .release-state/lock
expect_failure patch origin main
[[ ! -e events && ! -e tag ]]

new_fixture corrupt-plan
export FIXTURE_FAIL_TARGET=release-verify
expect_failure patch origin main
unset FIXTURE_FAIL_TARGET
printf 'unexpected record\n' >> .release-state/0.1.1.plan
expect_failure resume 0.1.1 origin main
[[ "$(count_event commit)" == 0 && "$(count_event push)" == 0 ]]

new_fixture changelog
cat > CHANGELOG.md <<'NOTES'
# Changelog

## [Draft]

- Current completed change.

## [0.1.0] - 2026-10-05

- Retained history.
NOTES
awk -v version=0.1.1 -v date=2026-10-06 -f "$ROOT/scripts/ci/finalize-release-changelog.awk" CHANGELOG.md > prepared
sed 's/## \[Draft\]/## [0.1.1] - 2026-10-06/' CHANGELOG.md > expected
cmp expected prepared
printf '\n## [0.2.0]\n\n- Competing selection.\n' >> CHANGELOG.md
if awk -v version=0.1.1 -v date=2026-10-06 -f "$ROOT/scripts/ci/finalize-release-changelog.awk" CHANGELOG.md > prepared 2>/dev/null; then exit 1; fi

[[ "$(bash "$ROOT/scripts/ci/next-release-version.sh" 9.8.7 patch)" == 9.8.8 ]]
[[ "$(bash "$ROOT/scripts/ci/next-release-version.sh" 9.8.7 minor)" == 9.9.0 ]]
[[ "$(bash "$ROOT/scripts/ci/next-release-version.sh" 9.8.7 major)" == 10.0.0 ]]
for version in 01.2.3 1.2.3-beta 1.2 9223372036854775807.0.0; do
    if bash "$ROOT/scripts/ci/next-release-version.sh" "$version" patch > /dev/null 2>&1; then exit 1; fi
done
echo 'release runner command-stub tests passed'
