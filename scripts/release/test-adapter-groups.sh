#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd -P)"
export PATH="$root/.tools/host/bin:$root/.tools/rust/common/bin:$PATH"
mkdir -p "$root/target/tmp"
fixture="$(mktemp -d "$root/target/tmp/canic-adapter-groups.XXXXXX")"
current_log=''
cleanup() {
    local status=$?
    if [[ "$status" == 0 ]]; then
        rm -rf -- "$fixture" || status=$?
    else
        [[ ! -f "$current_log" ]] || tail -65 "$current_log" >&2
        echo "Adapter group evidence retained: $fixture" >&2
    fi
    exit "$status"
}
trap cleanup EXIT
mkdir -p "$fixture/tmp"
export TMPDIR="$fixture/tmp"
editor="$(bash "$root/scripts/dev/release-metadata-editor.sh" --check)"
mkdir -p "$fixture/base/ci" "$fixture/base/scripts/dev" "$fixture/base/scripts/release"
scripts=(scripts/release/selection.sh scripts/release/metadata.sh scripts/release/render.sh
    scripts/release/rewrite-owned-lock.sh scripts/release/adapter.sh scripts/release/stage.sh
    scripts/ci/read-workspace-version.sh scripts/ci/bump-version.sh scripts/ci/sync-release-surface-version.sh
    scripts/ci/check-release-candidate.sh scripts/ci/check-release-surface-content.sh
    scripts/ci/check-release-draft-ready.sh scripts/ci/check-release-index.sh scripts/ci/read-release-validation.sh
    scripts/ci/cargo-release-group.sh scripts/ci/cargo-release-metadata.sh scripts/ci/check-release-source.sh
    scripts/ci/check-release-tag.sh scripts/ci/next-release-version.sh
    scripts/dev/report-release-cadence.sh scripts/ci/run-release.sh
    scripts/release/validate.sh scripts/release/validation-plan.pl scripts/release/validation-evidence.pl
    scripts/ci/run-canic-validation-targets.sh scripts/ci/run-validation-targets.sh)
bash "$root/scripts/ci/prepare-script-fixture.sh" "$root" "$fixture/base" "${scripts[@]}"
cp "$root/ci/"{release-validation.json,release-validation-scopes.json} "$fixture/base/ci/"
printf 'target/\n' > "$fixture/base/.gitignore"
printf '#!/usr/bin/env bash\nprintf "%%s\\n" %q\n' "$editor" > "$fixture/base/scripts/dev/release-metadata-editor.sh"
cat > "$fixture/base/Cargo.toml" <<'TOML'
[workspace]
members = ["crates/canic", "crates/canic-cli", "crates/canic-host", "crates/canic-macros", "crates/canic-icydb", "crates/canic-blob-service", "crates/probe"]
resolver = "3"
[workspace.package]
edition = "2024"
rust-version = "1.99.1"
[workspace.dependencies]
canic = { path = "crates/canic", version = "1.2.3" }
canic-cli = { path = "crates/canic-cli", version = "1.2.3" }
canic-host = { path = "crates/canic-host", version = "1.2.3" }
canic-macros = { path = "crates/canic-macros", version = "1.2.3" }
probe = { path = "crates/probe", version = "1.2.3" }
canic-icydb = { path = "crates/canic-icydb", version = "1.2.3" }
canic-blob-service = { path = "crates/canic-blob-service", version = "1.2.3" }
TOML
for package in canic canic-cli canic-host canic-macros canic-icydb canic-blob-service probe; do
    mkdir -p "$fixture/base/crates/$package/src"
    printf '[package]\nname = "%s"\nversion = "1.2.3"\nedition.workspace = true\nrust-version.workspace = true\n' "$package" > "$fixture/base/crates/$package/Cargo.toml"
    printf 'pub fn value() -> u8 { 1 }\n' > "$fixture/base/crates/$package/src/lib.rs"
done
printf 'publish = false\n[dependencies]\ncanic-icydb.workspace = true\ncanic-blob-service.workspace = true\n' >> "$fixture/base/crates/probe/Cargo.toml"
printf '[dependencies]\ncanic-macros.workspace = true\n' >> "$fixture/base/crates/canic-icydb/Cargo.toml"
printf '[dependencies]\ncanic.workspace = true\n' >> "$fixture/base/crates/canic-blob-service/Cargo.toml"
printf '[dependencies]\ncanic.workspace = true\n' >> "$fixture/base/crates/canic-cli/Cargo.toml"
jq '.groups |= map(if .name == "framework" then .members = ["canic","canic-macros","probe"] | .dependencies = .members else . end)' \
    "$root/ci/cargo-release-groups.json" > "$fixture/base/ci/cargo-release-groups.json"
groups=()
while IFS= read -r group; do groups+=("$group"); done < <(jq -er '.groups[].name' "$fixture/base/ci/cargo-release-groups.json")
for group in "${groups[@]}"; do
    mkdir -p "$fixture/base/releases/$group"
    printf '# Fixture\n\n## [1.2.4]\n\n### Changed\n\n- Independent group qualification.\n' > "$fixture/base/releases/$group/CHANGELOG.md"
    printf '{"schema":1,"group":"%s","source":null,"version":"1.2.3","date":null,"gate":null,"policy":null}\n' "$group" > "$fixture/base/releases/$group/release-validation.json"
done
printf 'CANIC_CLI_VERSION="%s"\n' '${CANIC_CLI_VERSION:-1.2.3}' > "$fixture/base/scripts/dev/install_dev.sh"
for consumer in consumer embedded-consumer; do
    path="$fixture/base/integrations/blob-service/$consumer"
    mkdir -p "$path/src"
    printf '[workspace]\n[package]\nname = "%s"\nversion = "0.1.0"\nedition = "2024"\n[dependencies]\ncanic = { path = "../../../crates/canic" }\ncanic-blob-service = { path = "../../../crates/canic-blob-service" }\n' "$consumer" > "$path/Cargo.toml"
    printf 'pub fn value() -> u8 { canic::value() }\n' > "$path/src/lib.rs"
    cargo generate-lockfile --offline --manifest-path "$path/Cargo.toml" > /dev/null
done
cargo generate-lockfile --offline --manifest-path "$fixture/base/Cargo.toml" > /dev/null
cargo sort --workspace --grouped "$fixture/base" > /dev/null
for group in "${groups[@]}"; do
    path="$fixture/$group"
    cp -R "$fixture/base" "$path"
    current_log="$fixture/$group.log"
    (
        cd "$path"
        git init -q
        git config user.name 'Adapter release fixture'
        git config user.email 'fixture@example.invalid'
        git add -- .
        git commit -qm 'Qualified fixture source'
        source="$(git rev-parse HEAD)"
        export RELEASE_GROUP="$group" RELEASE_PREVIOUS=1.2.3 RELEASE_VERSION=1.2.4 RELEASE_DATE=2026-11-02
        export CANIC_RELEASE_VALIDATED=1 CANIC_RELEASE_VALIDATED_HEAD="$source" CANIC_RELEASE_VALIDATION_KIND=complete
        git clone -q --local . "$fixture/occupied-$group"
        (
            cd "$fixture/occupied-$group"
            git config user.name 'Adapter release fixture'
            git config user.email 'fixture@example.invalid'
            git tag -a "$group-v1.2.4" -m occupied
            git pack-refs --all --prune
            if bash scripts/ci/bump-version.sh patch; then exit 1; fi
            [[ -z "$(git status --porcelain)" ]]
        )
        # Fail only the post-install live Cargo observation, after all immutable
        # candidate renders succeeded. The outer transaction must restore bytes.
        mkdir -p "$fixture/fault-bin"
        real_cargo="$(command -v cargo)"
        printf '#!/usr/bin/env bash\nset -euo pipefail\nif [[ "${CANIC_FIXTURE_FAIL_LIVE:-0}" == 1 && "$PWD" == "$CANIC_FIXTURE_LIVE_ROOT" ]] && rg -q '\''^version = "1.2.4"$'\'' "$CANIC_FIXTURE_LIVE_MANIFEST"; then exit 47; fi\nexec %q "$@"\n' "$real_cargo" > "$fixture/fault-bin/cargo"
        chmod +x "$fixture/fault-bin/cargo"
        export CANIC_FIXTURE_LIVE_ROOT="$path"
        CANIC_FIXTURE_LIVE_MANIFEST="$(jq -r --arg group "$group" '.groups[] | select(.name == $group) | .primary' ci/cargo-release-groups.json)"
        export CANIC_FIXTURE_LIVE_MANIFEST="crates/$CANIC_FIXTURE_LIVE_MANIFEST/Cargo.toml"
        if PATH="$fixture/fault-bin:$PATH" CANIC_FIXTURE_FAIL_LIVE=1 bash scripts/ci/bump-version.sh patch; then
            echo 'Failed live metadata check was accepted' >&2; exit 1
        fi
        [[ -z "$(git status --porcelain)" ]]
        bash scripts/ci/bump-version.sh patch
        cargo metadata --locked --offline --no-deps --format-version 1 > "$fixture/$group-metadata.json"
        jq -e --slurpfile metadata "$fixture/$group-metadata.json" --arg group "$group" '
            .groups | all(.[]; . as $g | all(.members[]; . as $n |
                $metadata[0].packages[] | select(.name == $n) |
                .version == (if $g.name == $group then "1.2.4" else "1.2.3" end)))
        ' ci/cargo-release-groups.json > /dev/null
        for other in "${groups[@]}"; do
            if [[ "$other" != "$group" ]]; then
                cmp "releases/$other/CHANGELOG.md" "$fixture/base/releases/$other/CHANGELOG.md"
                cmp "releases/$other/release-validation.json" "$fixture/base/releases/$other/release-validation.json"
            fi
        done
        if [[ "$group" != host ]]; then cmp scripts/dev/install_dev.sh "$fixture/base/scripts/dev/install_dev.sh"; fi
        if [[ "$group" == host ]]; then
            rg -q -F '${CANIC_CLI_VERSION:-1.2.4}' scripts/dev/install_dev.sh
        fi
        if [[ "$group" == icydb || "$group" == host ]]; then
            for consumer in consumer embedded-consumer; do
                cmp "integrations/blob-service/$consumer/Cargo.lock" "$fixture/base/integrations/blob-service/$consumer/Cargo.lock"
            done
        fi
        bash scripts/ci/check-release-candidate.sh
        bash scripts/release/stage.sh
        bash scripts/ci/check-release-index.sh
        # A source feature change is outside the exact metadata transaction.
        printf '\n[features]\nunreviewed = []\n' >> crates/canic/Cargo.toml
        if bash scripts/ci/check-release-candidate.sh > "$fixture/refusal.log" 2>&1; then exit 1; fi
        cp "$fixture/base/crates/canic/Cargo.toml" crates/canic/Cargo.toml
        if [[ "$group" == framework ]]; then perl -0pi -e 's/1\.2\.3/1.2.4/g' crates/canic/Cargo.toml; fi
        git commit -qm "Release $group 1.2.4"
        release="$(git rev-parse HEAD)"
        git tag -a "$group-v1.2.4" -m "Release $group 1.2.4"
        bash scripts/ci/check-release-tag.sh "$release" 1.2.4 "$group"
        bash scripts/ci/check-release-candidate.sh --commit "$release"
        bash scripts/ci/read-release-validation.sh "$group-v1.2.4" 1.2.4 > /dev/null
        [[ "$(bash scripts/dev/report-release-cadence.sh --tsv)" == $'1.2\t1\t12\t2' ]]
        printf '\n// Unrelated working copy edit.\n' >> crates/canic/src/lib.rs
        bash scripts/ci/check-release-candidate.sh --commit "$release"
    ) > "$current_log" 2>&1
done
# Exercise this consumer's actual hooks through Shared's saved group plan. A
# prepared failure must resume the same bytes and group against a local bare
# remote. The fixture gate is synthetic; it is no product qualification receipt.
cp -R "$fixture/base" "$fixture/delivery"
mkdir -p "$fixture/delivery/make"
cp "$root/make/release.mk" "$root/make/execution.mk" "$fixture/delivery/make/"
cat > "$fixture/delivery/Makefile" <<'MAKE'
include make/release.mk
release-version:
	@bash scripts/ci/read-workspace-version.sh
release-preflight:
	@bash scripts/release/adapter.sh preflight
prepare-validation:
	@test "$(SCOPE)" = release
release-verify:
	@bash scripts/release/validate.sh
release-prepare-version:
	@CANIC_RELEASE_VALIDATED=1 CANIC_RELEASE_VALIDATED_HEAD="$(RELEASE_SOURCE)" CANIC_RELEASE_VALIDATION_KIND=plan CANIC_RELEASE_DATE="$(RELEASE_DATE)" bash scripts/ci/bump-version.sh "$(RELEASE_KIND)"
release-prepared-check:
	@bash scripts/ci/check-release-candidate.sh
	@test "$(INJECT_PREPARED_FAILURE)" != 1
release-files:
	@bash scripts/release/adapter.sh files
release-commit-check:
	@bash scripts/ci/check-release-index.sh
	@bash scripts/ci/check-release-candidate.sh
release-committed-check:
	@bash scripts/release/adapter.sh committed
release-tagged-check release-push-check:
	@bash scripts/release/adapter.sh tagged
MAKE
cat >> "$fixture/delivery/Makefile" <<'MAKE'
release-validation-roster:
	@jq -r '.stages[]' ci/release-validation.json
MAKE
while IFS= read -r target; do
    printf '%s:\n\t@printf "fixture gate executed: %%s\\n" "%s"\n' "$target" "$target" >> "$fixture/delivery/Makefile"
done < <(jq -r '.stages[]' "$root/ci/release-validation.json")
current_log="$fixture/resume.log"
(
    cd "$fixture/delivery"
    git init -q --initial-branch=main
    git config user.name 'Adapter release fixture'
    git config user.email 'fixture@example.invalid'
    git add -- .
    git commit -qm 'Source for consumer group delivery fixture'
    git tag -a blob-service-v1.2.3 -m 'Unqualified baseline'
    mkdir -p crates/canic-blob-service/tests
    printf '#[test]\nfn qualification_fixture() {}\n' > crates/canic-blob-service/tests/qualification.rs
    git add -- crates/canic-blob-service/tests/qualification.rs
    git commit -qm 'Group-confined consumer fixture change'
    git init -q --bare "$fixture/remote.git"
    git remote add origin "$fixture/remote.git"
    git push -q -u origin main
    if make --no-print-directory release-patch RELEASE_GROUP=blob-service INJECT_PREPARED_FAILURE=1; then exit 1; fi
    [[ -f .git/release-state/blob-service-1.2.4.plan ]]
    git diff --binary > "$fixture/prepared.diff"
    if make --no-print-directory release-resume RELEASE_GROUP=icydb VERSION=1.2.4; then exit 1; fi
    git diff --binary > "$fixture/wrong-group.diff"
    cmp "$fixture/prepared.diff" "$fixture/wrong-group.diff"
    log="$(jq -r '.validation.execution[0].log' releases/blob-service/release-validation.json)"
    cp "$log" "$fixture/retained-stage.log"
    rm -- "$log"
    if make --no-print-directory release-resume RELEASE_GROUP=blob-service VERSION=1.2.4; then
        echo 'Missing plan execution evidence was accepted during recovery' >&2; exit 1
    fi
    cp "$fixture/retained-stage.log" "$log"
    make --no-print-directory release-resume RELEASE_GROUP=blob-service VERSION=1.2.4
    release="$(git rev-parse HEAD)"
    [[ "$(git rev-parse 'blob-service-v1.2.4^{commit}')" == "$release" ]]
    [[ "$(git --git-dir="$fixture/remote.git" rev-parse refs/heads/main)" == "$release" ]]
    [[ "$(git --git-dir="$fixture/remote.git" rev-parse 'blob-service-v1.2.4^{commit}')" == "$release" ]]
    [[ "$(RELEASE_GROUP=blob-service bash scripts/ci/read-release-validation.sh blob-service-v1.2.4 1.2.4)" == *$'\tplan-workspace' ]]
    # The renderer's actual root/catalog/private-lock transaction must carry
    # qualification to the resulting graph, not just the toy selector graph.
    printf '\n// Subsequent confined source.\n' >> crates/canic-blob-service/tests/qualification.rs
    git add -- crates/canic-blob-service/tests/qualification.rs
    git commit -qm 'Confined change after canonical lock transaction'
    source="$(git rev-parse HEAD)"
    make --no-print-directory release-verify RELEASE_GROUP=blob-service RELEASE_SOURCE="$source"
    jq -e '.plan.scope == "group" and .context.qualification.tag == "blob-service-v1.2.4"' \
        "target/release-validation/blob-service/$source.json" > /dev/null
) > "$current_log" 2>&1
# Public Make entrypoints must refuse before prerequisites or release effects.
for selection in '' unknown; do
    for target in release-patch release-minor release-resume publish patch; do
        if RELEASE_GROUP="$selection" make -C "$root" --no-print-directory "$target" > "$fixture/entrypoint.log" 2>&1; then
            echo "Missing/unknown group accepted by $target" >&2; exit 1
        fi
        rg -q 'Select a valid explicit RELEASE_GROUP' "$fixture/entrypoint.log"
    done
done
echo 'Adapter groups preserve unselected identities, ledgers, consumer locks, exact candidate/tag provenance and explicit public selection'
