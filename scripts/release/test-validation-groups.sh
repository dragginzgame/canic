#!/usr/bin/env bash
set -euo pipefail
# Consumer dispatch/evidence regression only. Inert gates never establish real
# compiler, native host or installed qualification and are never exported.
unset MAKEFLAGS MFLAGS MAKEOVERRIDES GNUMAKEFLAGS MAKEFILES
unset VALIDATION_INPUT_ROOT VALIDATION_INPUT_BASELINE VALIDATION_LOG_DIR
root="$(cd "$(dirname "$0")/../.." && pwd -P)"
mkdir -p "$root/target/tmp"
fixture="$(mktemp -d "$root/target/tmp/canic-validation-groups.XXXXXX")"
trap 'if [[ $? == 0 ]]; then rm -rf -- "$fixture"; else echo "Group validation fixture retained: $fixture" >&2; fi' EXIT
mkdir -p "$fixture/base/scripts/release" "$fixture/base/ci" "$fixture/base/target"
mkdir -p "$fixture/base/docs/status"
printf 'Descriptive handoff\n' > "$fixture/base/docs/status/current.md"
bash "$root/scripts/ci/prepare-script-fixture.sh" "$root" "$fixture/base" scripts/ci/run-validation-targets.sh
cp "$root/scripts/ci/run-canic-validation-targets.sh" "$fixture/base/scripts/ci/"
cp "$root/scripts/release/"{selection.sh,validate.sh,validation-plan.pl,validation-evidence.pl} "$fixture/base/scripts/release/"
cp "$root/ci/"{cargo-release-groups.json,release-validation.json,release-validation-scopes.json} "$fixture/base/ci/"
printf 'target/\n' > "$fixture/base/.gitignore"
groups=()
mapfile -t groups < <(jq -r '.groups[].name' "$root/ci/cargo-release-groups.json")
for group in "${groups[@]}"; do
    primary="$(jq -r --arg group "$group" '.groups[] | select(.name == $group) | .primary' "$root/ci/cargo-release-groups.json")"
    mkdir -p "$fixture/base/crates/$primary" "$fixture/base/releases/$group"
    printf '[package]\nname = "%s"\nversion = "1.2.3"\n' "$primary" > "$fixture/base/crates/$primary/Cargo.toml"
    printf '# Group %s\n' "$group" > "$fixture/base/releases/$group/CHANGELOG.md"
    printf '{"schema":1,"group":"%s","version":"1.2.3","source":null,"gate":null,"policy":null,"date":null}\n' "$group" > "$fixture/base/releases/$group/release-validation.json"
done
cat > "$fixture/base/Makefile" <<'MAKE'
.PHONY: release-validation-roster
release-validation-roster:
	@jq -r '.stages[]' ci/release-validation.json
MAKE
while IFS= read -r stage; do
    printf '.PHONY: %s\n%s:\n\t@bash ./gate %s\n' "$stage" "$stage" "$stage" >> "$fixture/base/Makefile"
done < <(jq -r '.stages[]' "$root/ci/release-validation.json")
cat > "$fixture/base/gate" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$1" >> "$GROUP_TEST_EVENTS"
if [[ "$1" == clippy ]]; then
    [[ -z "${GROUP_TEST_DRIFT:-}" ]] || printf 'drift\n' >> "$GROUP_TEST_DRIFT"
    [[ "${GROUP_TEST_MOVE_HEAD:-0}" == 0 ]] || git commit --allow-empty -qm moved
fi
[[ "$1" != "${GROUP_TEST_FAIL:-}" ]]
SH
mkdir -p "$fixture/base/crates/canic-cli/src/output" "$fixture/base/crates/canic-icydb/tests" "$fixture/base/crates/canic-blob-service/src/ops/metrics"
printf 'one\n' > "$fixture/base/crates/canic-cli/src/output/mod.rs"
printf 'one\n' > "$fixture/base/crates/canic-icydb/tests/participant.rs"
printf 'one\n' > "$fixture/base/crates/canic-blob-service/src/ops/metrics/mod.rs"
(
    cd "$fixture/base"
    git init -q
    git config user.name 'Canic validation fixture'
    git config user.email 'fixture@example.invalid'
    git add -- .
    git commit -qm baseline
    for group in "${groups[@]}"; do git tag -a "$group-v1.2.3" -m baseline; done
)
run_gate() {
    export RELEASE_SOURCE
    RELEASE_SOURCE="$(git rev-parse HEAD)"
    export GROUP_TEST_EVENTS="$PWD/target/events"
    : > "$GROUP_TEST_EVENTS"
    bash scripts/release/validate.sh "$@" > target/attempt.log 2>&1 || { cat target/attempt.log >&2; return 1; }
    proof="target/release-validation/$RELEASE_GROUP/$RELEASE_SOURCE.json"
    perl scripts/release/validation-evidence.pl verify "$proof" > /dev/null
}
reject() {
    if "$@" > target/rejection.log 2>&1; then echo "Expected refusal: $*" >&2; exit 1; fi
}
publish_fixture_version() {
    local version="$1" primary
    primary="$(jq -r --arg group "$RELEASE_GROUP" '.groups[] | select(.name == $group) | .primary' ci/cargo-release-groups.json)"
    sed -E "s/^version = .*/version = \"$version\"/" "crates/$primary/Cargo.toml" > target/manifest
    cat target/manifest > "crates/$primary/Cargo.toml"
    RELEASE_VERSION="$version" perl scripts/release/validation-evidence.pl project "$proof" . > target/qualification.json
    jq --arg source "$RELEASE_SOURCE" --arg group "$RELEASE_GROUP" --arg version "$version" \
        --arg policy "$(git rev-parse HEAD:ci/cargo-release-groups.json)" --slurpfile qualification target/qualification.json \
        '{schema:1,group:$group,version:$version,source:$source,date:"2026-11-03",gate:"plan",policy:$policy,
            validation:.,qualification:$qualification[0]}' "$proof" > "releases/$RELEASE_GROUP/release-validation.json"
    git add -- "crates/$primary/Cargo.toml" "releases/$RELEASE_GROUP/release-validation.json"
    git commit -qm "Release $RELEASE_GROUP $version"
    git tag -a "$RELEASE_GROUP-v$version" -m 'Synthetic dispatch qualification'
}
for group in "${groups[@]}"; do
    cp -R "$fixture/base" "$fixture/$group"
    (
        cd "$fixture/$group"
        export RELEASE_GROUP="$group"
        case "$group" in
            host) changed=crates/canic-cli/src/output/mod.rs ;;
            icydb) changed=crates/canic-icydb/tests/participant.rs ;;
            blob-service) changed=crates/canic-blob-service/src/ops/metrics/mod.rs ;;
            framework) changed=releases/framework/CHANGELOG.md ;;
        esac
        printf 'two\n' >> "$changed"
        printf 'Updated descriptive handoff\n' >> docs/status/current.md
        git add -- "$changed" docs/status/current.md
        git commit -qm 'Selected group change'
        run_gate
        jq -e '.plan.scope == "workspace"' "$proof" > /dev/null
        jq -r '.stages[]' ci/release-validation.json > target/full-roster
        cmp target/full-roster "$GROUP_TEST_EVENTS"
        # Compare the proposed subset against successful execution of the full
        # roster on this exact source. Production only obtains this evidence by
        # executing the actual existing Canic stage recipes, never these stubs.
        publish_fixture_version 1.2.4
        printf 'three\n' >> "$changed"
        git add -- "$changed"
        git commit -qm 'Subsequent selected group change'
        run_gate
        if [[ "$group" == framework ]]; then
            jq -e '.plan.scope == "workspace"' "$proof" > /dev/null
        else
            jq -e '.plan.scope == "group"' "$proof" > /dev/null
            jq -r '.plan.stages[] | select(.required) | .target' "$proof" > target/expected
            cmp target/expected "$GROUP_TEST_EVENTS"
            excluded="$(jq -r '.plan.stages[] | select(.required == false) | .target' "$proof" | head -1)"
            GROUP_TEST_FAIL="$excluded" run_gate
            # Every configured omission is absent from the command trace.
            while IFS= read -r omitted; do
                if rg -Fxq "$omitted" "$GROUP_TEST_EVENTS"; then echo "Excluded stage executed: $omitted" >&2; exit 1; fi
            done < <(jq -r '.plan.stages[] | select(.required == false) | .target' "$proof")
            if [[ "$group" == host ]]; then
                run_gate --workspace
                jq -e '.plan.scope == "workspace"' "$proof" > /dev/null
                cmp target/full-roster "$GROUP_TEST_EVENTS"
                run_gate
                # Canonical version metadata advances graph bytes while carrying
                # the same full basis; a second narrow release remains eligible.
                publish_fixture_version 1.2.5
                printf 'four\n' >> "$changed"
                git add -- "$changed"
                git commit -qm 'Still confined after narrow release'
                run_gate
                jq -e '.plan.scope == "group" and .context.qualification.tag == "host-v1.2.4"' "$proof" > /dev/null
            fi
        fi
        cp "$proof" target/good-proof.json
        reject env RUSTFLAGS=-Cdebuginfo=1 perl scripts/release/validation-evidence.pl verify "$proof"
        if [[ "$group" == host ]]; then
            object="$(git rev-parse refs/tags/host-v1.2.4)"
            git tag -fa host-v1.2.4 -m 'Moved baseline must refuse' > /dev/null
            reject perl scripts/release/validation-evidence.pl verify "$proof"
            git update-ref refs/tags/host-v1.2.4 "$object"
        fi
        jq 'del(.execution[-1])' target/good-proof.json > target/incomplete.json
        reject perl scripts/release/validation-evidence.pl verify target/incomplete.json
        log="$(jq -r '.execution[0].log' "$proof")"
        cp "$log" target/log-backup
        printf 'tampered\n' >> "$log"
        reject perl scripts/release/validation-evidence.pl verify "$proof"
        cat target/log-backup > "$log"
        # A missing/interrupted gate cannot authorize preparation, and a failed
        # new attempt cannot fall back to its predecessor's successful pointer.
        reject env GROUP_TEST_FAIL=clippy bash scripts/release/validate.sh
        [[ ! -e "$proof" ]]
        reject perl scripts/release/validation-evidence.pl verify "$proof"
        run_gate
        if [[ "$group" == host ]]; then
            # A real feature/dependency input change cannot carry qualification
            # merely because the recipes and an older full tag still match.
            printf '\n[features]\nnew-input = []\n' >> crates/canic-cli/Cargo.toml
            git add -- crates/canic-cli/Cargo.toml
            git commit -qm 'New graph requires qualification'
            run_gate
            jq -e '.plan.scope == "workspace" and .context.candidate_scope == "workspace"' "$proof" > /dev/null
            publish_fixture_version 1.2.6
            printf 'five\n' >> "$changed"
            git add -- "$changed"
            git commit -qm 'Confined source needs new graph qualification'
            run_gate
            jq -e '.plan.scope == "workspace" and .context.candidate_scope == "group" and .context.qualification == null' "$proof" > /dev/null
        fi
        cp "$proof" target/prior-source.json
        printf 'shared\n' > shared-input
        git add -- shared-input
        git commit -qm 'Shared input broadens'
        run_gate
        jq -e '.plan.scope == "workspace" and all(.plan.stages[]; .required)' "$proof" > /dev/null
        reject perl scripts/release/validation-evidence.pl verify target/prior-source.json
        reject env GROUP_TEST_DRIFT="$changed" bash scripts/release/validate.sh
        git restore -- "$changed"
        reject env GROUP_TEST_MOVE_HEAD=1 bash scripts/release/validate.sh
        # A new stage cannot disappear from the selector's roster.
        jq -cS '.stages += ["new-obligation"]' ci/release-validation.json > target/policy
        cat target/policy > ci/release-validation.json
        git add -- ci/release-validation.json
        git commit -qm 'New stage'
        export RELEASE_SOURCE
        RELEASE_SOURCE="$(git rev-parse HEAD)"
        reject bash scripts/release/validate.sh
        reject env RELEASE_GROUP=unknown bash scripts/release/validate.sh
        reject env CANIC_TEST_PLAN_ONLY=1 bash scripts/release/validate.sh
        # Group validation itself never mutates any package version or ledger.
        for other in "${groups[@]}"; do
            [[ "$other" == "$group" ]] && continue
            cmp "releases/$other/CHANGELOG.md" "$fixture/base/releases/$other/CHANGELOG.md"
            cmp "releases/$other/release-validation.json" "$fixture/base/releases/$other/release-validation.json"
            primary="$(jq -r --arg group "$other" '.groups[] | select(.name == $group) | .primary' ci/cargo-release-groups.json)"
            cmp "crates/$primary/Cargo.toml" "$fixture/base/crates/$primary/Cargo.toml"
        done
        echo "$group: full fallback, qualified subset, failure/drift/refusal and retained proof passed"
    )
done
echo 'Canic group routing/evidence fixtures passed; actual full/native/installed qualification remains release-owned'
