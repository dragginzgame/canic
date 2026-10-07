#!/usr/bin/env bash
set -euo pipefail

# Real Cargo/archive records with fake Git: no commits, tags, pushes or builds.
root="$(cd "$(dirname "$0")/../.." && pwd -P)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-commit-view.XXXXXX")"
trap 'if [[ $? == 0 ]]; then rm -rf -- "$fixture"; else printf "Failed commit-view fixture retained: %s\n" "$fixture" >&2; fi' EXIT
mkdir -p "$fixture/source/src" "$fixture/source/scripts/dev" \
    "$fixture/source/docs/changelog" "$fixture/bin" "$fixture/scripts/ci" "$fixture/scripts/release"
cp "$root/scripts/ci/check-release-candidate.sh" "$root/scripts/ci/check-release-surface-content.sh" \
    "$root/scripts/ci/require-jq.sh" "$root/scripts/ci/check-release-tag.sh" \
    "$root/scripts/ci/read-cargo-workspace-version.sh" "$root/scripts/ci/rewrite-local-lock-versions.pl" "$fixture/scripts/ci/"
cp "$root/scripts/release/adapter.sh" "$root/scripts/release/rewrite-owned-lock.sh" "$fixture/scripts/release/"
cat > "$fixture/source/Cargo.toml" <<'TOML'
[workspace]
resolver = "3"
[workspace.package]
version = "1.2.3"
[package]
name = "release-view-fixture"
version.workspace = true
edition = "2024"
TOML
printf 'pub fn fixture() {}\n' > "$fixture/source/src/lib.rs"
printf 'CANIC_CLI_VERSION="${CANIC_CLI_VERSION:-1.2.3}"\n' > "$fixture/source/scripts/dev/install_dev.sh"
printf '# Fixture\n\n## [1.2.4]\n' > "$fixture/source/docs/changelog/1.2.md"
cargo generate-lockfile --offline --manifest-path "$fixture/source/Cargo.toml" > "$fixture/cargo.log" 2>&1
cp -R "$fixture/source" "$fixture/candidate"
cargo set-version --workspace --offline --manifest-path "$fixture/candidate/Cargo.toml" 1.2.4 >> "$fixture/cargo.log" 2>&1
printf 'CANIC_CLI_VERSION="${CANIC_CLI_VERSION:-1.2.4}"\n' > "$fixture/candidate/scripts/dev/install_dev.sh"
printf '# Fixture\n\n## [1.2.4] - 2026-10-06\n' > "$fixture/candidate/docs/changelog/1.2.md"
export VIEW_SOURCE=1111111111111111111111111111111111111111
export VIEW_COMMIT=2222222222222222222222222222222222222222
export VIEW_FIXTURE="$fixture" VIEW_EVENTS="$fixture/events"
printf '{"schema":1,"version":"1.2.4","source":"%s","date":"2026-10-06","gate":"complete"}\n' \
    "$VIEW_SOURCE" > "$fixture/candidate/release-validation.json"
cp "$fixture/source/Cargo.toml" "$fixture/Cargo.toml"
# The working metadata deliberately disagrees with the retained release.
sed 's/1.2.3/1.2.9/' "$fixture/Cargo.toml" > "$fixture/changed"
mv "$fixture/changed" "$fixture/Cargo.toml"
cat > "$fixture/bin/git" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == -C ]]; then shift 2; fi
printf '%s\n' "$*" >> "$VIEW_EVENTS"
case "$*" in
    "rev-parse --verify $VIEW_COMMIT^{commit}") echo "$VIEW_COMMIT" ;;
    "log -1 --format=%s $VIEW_COMMIT") echo 'Release 1.2.4' ;;
    "rev-parse $VIEW_COMMIT^") echo "$VIEW_SOURCE" ;;
    "diff --name-only $VIEW_SOURCE $VIEW_COMMIT --") printf '%s\n' Cargo.toml Cargo.lock scripts/dev/install_dev.sh release-validation.json docs/changelog/1.2.md ;;
    "cat-file -e $VIEW_SOURCE:"*) exit 0 ;;
    "archive $VIEW_SOURCE") tar -cf - -C "$VIEW_FIXTURE/source" . ;;
    "archive $VIEW_COMMIT") tar -cf - -C "$VIEW_FIXTURE/candidate" . ;;
    'cat-file -t refs/tags/v1.2.4') echo "${VIEW_TAG_TYPE:-tag}" ;;
    'rev-parse --verify refs/tags/v1.2.4^{commit}') echo "${VIEW_TAG_COMMIT:-$VIEW_COMMIT}" ;;
    *) echo "unexpected Git observation: $*" >&2; exit 99 ;;
esac
SH
chmod +x "$fixture/bin/git"
export PATH="$fixture/bin:$PATH"
export RELEASE_COMMIT="$VIEW_COMMIT" RELEASE_VERSION=1.2.4 RELEASE_DATE=2026-10-06
cd "$fixture"
expect_failure() {
    if "$@" > "$fixture/output" 2>&1; then
        echo 'commit-view test accepted mismatched release authority' >&2; exit 1
    fi
}
bash scripts/release/adapter.sh committed > "$fixture/output" 2>&1
bash scripts/release/adapter.sh tagged > "$fixture/output" 2>&1
RELEASE_VERSION=1.2.5 expect_failure bash scripts/release/adapter.sh committed
RELEASE_DATE=2026-10-07 expect_failure bash scripts/release/adapter.sh committed
VIEW_TAG_COMMIT="$VIEW_SOURCE" expect_failure bash scripts/release/adapter.sh tagged
VIEW_TAG_TYPE=commit expect_failure bash scripts/release/adapter.sh tagged
cp candidate/release-validation.json original-receipt
printf '{"schema":1,"version":"1.2.4","source":"%s","date":"2026-10-06","gate":"complete"}\n' \
    "$VIEW_COMMIT" > candidate/release-validation.json
expect_failure bash scripts/release/adapter.sh committed
cp original-receipt candidate/release-validation.json
printf '\n[features]\nunvalidated = []\n' >> candidate/Cargo.toml
expect_failure bash scripts/release/adapter.sh committed
if rg -q 'HEAD' "$VIEW_EVENTS"; then echo 'commit view unexpectedly observed HEAD' >&2; exit 1; fi
echo 'selected release commit, source receipt, payload, version/date and exact tag checks passed (fake Git)'
