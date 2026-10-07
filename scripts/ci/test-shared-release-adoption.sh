#!/usr/bin/env bash
set -euo pipefail

# Execute the real version transaction in toy workspaces with fake Git authority.
# No source repository commits, refs, index writes or package versions change.
ROOT="$(cd "$(dirname "$0")/../.." && pwd -P)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-shared-release.XXXXXX")"
trap 'if [[ $? == 0 ]]; then rm -rf -- "$fixture"; else echo "Release adoption evidence retained: $fixture" >&2; fi' EXIT
mkdir -p "$fixture/base/scripts/ci" "$fixture/base/scripts/release" \
    "$fixture/base/scripts/dev" "$fixture/base/docs/changelog" "$fixture/base/member/src" "$fixture/bin"
for name in bump-version require-jq read-workspace-version read-cargo-workspace-version \
    next-release-version check-release-draft-ready sync-release-surface-version; do
    cp "$ROOT/scripts/ci/$name.sh" "$fixture/base/scripts/ci/"
done
cp "$ROOT/scripts/ci/rewrite-local-lock-versions.pl" "$ROOT/scripts/ci/finalize-release-changelog.awk" "$fixture/base/scripts/ci/"
cp "$ROOT/scripts/release/rewrite-owned-lock.sh" "$fixture/base/scripts/release/"
cat > "$fixture/base/Cargo.toml" <<'TOML'
[workspace]
members = ["member"]
resolver = "3"
[workspace.package]
version = "1.2.3"
TOML
cat > "$fixture/base/member/Cargo.toml" <<'TOML'
[package]
name = "release-adoption-fixture"
version.workspace = true
edition = "2024"
TOML
printf 'pub fn fixture() {}\n' > "$fixture/base/member/src/lib.rs"
printf 'CANIC_CLI_VERSION="${CANIC_CLI_VERSION:-1.2.3}"\n' > "$fixture/base/scripts/dev/install_dev.sh"
for notes in CHANGELOG.md docs/changelog/1.2.md; do
    printf '# Fixture\n\n## [1.2.4]\n\n- Current batch.\n\n## [1.2.3]\n\n- Retained undated history.\n' > "$fixture/base/$notes"
done
cargo generate-lockfile --offline --manifest-path "$fixture/base/Cargo.toml" > "$fixture/cargo.log" 2>&1
cat > "$fixture/bin/git" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
case "$*" in
    'rev-parse --show-toplevel') echo "$RELEASE_TEST_ROOT" ;;
    'rev-parse HEAD') echo 1111111111111111111111111111111111111111 ;;
    'status --porcelain') ;;
    'ls-files -- '* ) printf '%s\n' Cargo.toml member/Cargo.toml ;;
    'rev-parse v1.2.4') exit 1 ;;
    *) echo "unexpected Git effect/observation: $*" >&2; exit 99 ;;
esac
SH
chmod +x "$fixture/bin/git"
export PATH="$fixture/bin:$PATH"
export CANIC_RELEASE_VALIDATED=1 CANIC_RELEASE_VALIDATED_HEAD=1111111111111111111111111111111111111111
export RELEASE_VERSION=1.2.4 CANIC_RELEASE_DATE=2026-10-06 CANIC_RELEASE_VALIDATION_KIND=complete
for mode in absent replace rollback rollback-absent partial-output; do
    cp -R "$fixture/base" "$fixture/$mode"
    export RELEASE_TEST_ROOT="$fixture/$mode"
    if [[ "$mode" != absent && "$mode" != rollback-absent ]]; then printf '{"previous":"receipt"}\n' > "$RELEASE_TEST_ROOT/release-validation.json"; fi
    if [[ "$mode" == rollback || "$mode" == rollback-absent ]]; then
        printf '#!/usr/bin/env bash\nexit 23\n' > "$RELEASE_TEST_ROOT/scripts/ci/sync-release-surface-version.sh"
    elif [[ "$mode" == partial-output ]]; then
        printf '#!/usr/bin/env perl\nprint "partial candidate"; exit 19;\n' > "$RELEASE_TEST_ROOT/scripts/ci/rewrite-local-lock-versions.pl"
    fi
    status=0
    (cd "$RELEASE_TEST_ROOT"; bash scripts/ci/bump-version.sh patch) > "$fixture/$mode.log" 2>&1 || status=$?
    if [[ "$mode" == rollback || "$mode" == rollback-absent || "$mode" == partial-output ]]; then
        expected=23; [[ "$mode" != partial-output ]] || expected=19
        [[ "$status" == "$expected" ]] || { cat "$fixture/$mode.log" >&2; exit 1; }
        for path in Cargo.toml member/Cargo.toml Cargo.lock CHANGELOG.md docs/changelog/1.2.md scripts/dev/install_dev.sh; do
            cmp "$fixture/base/$path" "$RELEASE_TEST_ROOT/$path"
        done
        if [[ "$mode" == rollback-absent ]]; then [[ ! -e "$RELEASE_TEST_ROOT/release-validation.json" ]]
        else [[ "$(cat "$RELEASE_TEST_ROOT/release-validation.json")" == '{"previous":"receipt"}' ]]; fi
    else
        [[ "$status" == 0 ]] || { cat "$fixture/$mode.log" >&2; exit 1; }
        jq -e 'keys == ["date","gate","schema","source","version"] and .schema == 1 and
            .source == "1111111111111111111111111111111111111111" and .version == "1.2.4" and
            .date == "2026-10-06" and .gate == "complete"' "$RELEASE_TEST_ROOT/release-validation.json" >/dev/null
        cargo metadata --locked --offline --no-deps --format-version 1 --manifest-path "$RELEASE_TEST_ROOT/Cargo.toml" \
            | jq -e 'all(.packages[]; .version == "1.2.4")' >/dev/null
        for notes in CHANGELOG.md docs/changelog/1.2.md; do
            grep -Fx '## [1.2.4] - 2026-10-06' "$RELEASE_TEST_ROOT/$notes" >/dev/null
            grep -Fx '## [1.2.3]' "$RELEASE_TEST_ROOT/$notes" >/dev/null
        done
    fi
done
echo 'Shared release transaction, receipt replacement and exact rollback passed (real Cargo; fake Git)'
