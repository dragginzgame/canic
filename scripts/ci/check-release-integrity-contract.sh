#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CI="$ROOT/.github/workflows/ci.yml"
CODEOWNERS="$ROOT/.github/CODEOWNERS"
MAKEFILE="$ROOT/Makefile"
TOOLS="$ROOT/tool-versions.env"
RUST_TOOLCHAIN="$ROOT/rust-toolchain.toml"
MATRIX="$ROOT/docs/governance/supported-platforms.md"
VERIFY="$ROOT/scripts/ci/verify-file-checksum.sh"
ICP_REQUIRE="$ROOT/scripts/ci/require_icp.sh"
ICP_MODEL="$ROOT/crates/canic-host/src/icp/model.rs"
ICP_PROOF="$ROOT/scripts/ci/blob-storage-cli-proof-lib.sh"
DEV_INSTALL="$ROOT/scripts/dev/install_dev.sh"
GIT_HOOK_INSTALLER="$ROOT/scripts/dev/install-git-hooks.sh"
PRE_COMMIT_HOOK="$ROOT/.githooks/pre-commit"
ICP_UPDATE="$ROOT/scripts/dev/update-icp-cli-pin.sh"
INSTALLING="$ROOT/INSTALLING.md"
README="$ROOT/README.md"
SECRET_SCAN="$ROOT/scripts/ci/run-secret-scan.sh"
GITLEAKS_IGNORE="$ROOT/.gitleaksignore"
DEPENDENCY_RISK_GATE="$ROOT/scripts/ci/check-dependency-risk-inventory.sh"
DEPENDENCY_RISK_TEST="$ROOT/scripts/ci/test-dependency-risk-inventory.sh"
DEPENDENCY_RISK_INVENTORY="$ROOT/scripts/ci/dependency-risk-inventory.tsv"
BUMP_VERSION="$ROOT/scripts/ci/bump-version.sh"
CONFIRM_VERSION_BUMP="$ROOT/scripts/ci/confirm-version-bump.sh"
RELEASE_CANDIDATE="$ROOT/scripts/ci/check-release-candidate.sh"
RELEASE_CADENCE="$ROOT/scripts/dev/report-release-cadence.sh"
VERSION_READER="$ROOT/scripts/ci/read-workspace-version.sh"
PUBLISH_WORKSPACE="$ROOT/scripts/ci/publish-workspace.sh"
RELEASE_CLEANUP="$ROOT/scripts/ci/cleanup-release-artifacts.sh"
TEST_SCRATCH_RUNNER="$ROOT/scripts/ci/run-with-test-scratch.sh"
POCKET_IC_STOPPER="$ROOT/scripts/ci/stop-owned-pocketic-servers.sh"
RELEASE_PUSH_READY="$ROOT/scripts/ci/check-release-push-ready.sh"
RELEASE_PUSH="$ROOT/scripts/ci/push-release.sh"
POCKET_IC_ALIGNMENT="$ROOT/scripts/ci/check-pocketic-version-alignment.sh"
WORKSPACE_TEST_INVENTORY="$ROOT/scripts/ci/workspace-test-inventory.tsv"
WORKSPACE_TEST_INVENTORY_GATE="$ROOT/scripts/ci/check-workspace-test-inventory.sh"
WORKSPACE_TEST_RUNNER="$ROOT/scripts/ci/run-workspace-tests.sh"
TAG_DELETE_TEST="$ROOT/scripts/ci/test-delete-github-tags-up-to.sh"
installers=(
    "$ROOT/scripts/ci/install-actionlint.sh"
    "$ROOT/scripts/ci/install-gitleaks.sh"
    "$ROOT/scripts/ci/install-shellcheck.sh"
    "$ROOT/scripts/ci/install-pocketic.sh"
    "$ROOT/scripts/ci/install-icp-cli.sh"
    "$ROOT/scripts/ci/install-ic-wasm.sh"
)

fail() {
    echo "release integrity guard failed: $1" >&2
    exit 1
}

for file in "$CI" "$CODEOWNERS" "$MAKEFILE" "$TOOLS" "$RUST_TOOLCHAIN" "$MATRIX" "$VERIFY" "$ICP_REQUIRE" "$ICP_MODEL" "$ICP_PROOF" "$DEV_INSTALL" "$GIT_HOOK_INSTALLER" "$PRE_COMMIT_HOOK" "$ICP_UPDATE" "$INSTALLING" "$README" "$SECRET_SCAN" "$GITLEAKS_IGNORE" "$DEPENDENCY_RISK_GATE" "$DEPENDENCY_RISK_TEST" "$DEPENDENCY_RISK_INVENTORY" "$BUMP_VERSION" "$CONFIRM_VERSION_BUMP" "$RELEASE_CANDIDATE" "$RELEASE_CADENCE" "$VERSION_READER" "$PUBLISH_WORKSPACE" "$RELEASE_CLEANUP" "$TEST_SCRATCH_RUNNER" "$POCKET_IC_STOPPER" "$RELEASE_PUSH_READY" "$RELEASE_PUSH" "$POCKET_IC_ALIGNMENT" "$WORKSPACE_TEST_INVENTORY" "$WORKSPACE_TEST_INVENTORY_GATE" "$WORKSPACE_TEST_RUNNER" "$TAG_DELETE_TEST"; do
    [ -f "$file" ] || fail "missing required file: $file"
done

external_action_count=0
while IFS= read -r uses_entry; do
    action="${uses_entry#uses:}"
    action="${action#"${action%%[![:space:]]*}"}"
    case "$action" in
    ./*) continue ;;
    esac
    external_action_count=$((external_action_count + 1))
    if [[ ! "$action" =~ @[0-9a-f]{40}$ ]]; then
        fail "external Action is not pinned to a full commit: $action"
    fi
done < <(rg -o --no-filename 'uses:[[:space:]]*[^[:space:]#]+' "$ROOT/.github/workflows" -g '*.yml' -g '*.yaml')

[ "$external_action_count" -gt 0 ] || fail "no external Actions were inspected"

mapfile -t workflow_files < <(
    find "$ROOT/.github/workflows" -maxdepth 1 -type f \
        \( -name '*.yml' -o -name '*.yaml' \) | sort
)
for workflow_file in "${workflow_files[@]}"; do
    rg -q '^permissions:$' "$workflow_file" ||
        fail "workflow does not declare top-level token permissions: $workflow_file"
done
if rg -n 'runs-on:[[:space:]]+ubuntu-latest' "${workflow_files[@]}" >/dev/null; then
    fail "workflow runners must use the fixed Ubuntu 24.04 image"
fi
checkout_count="$(rg -o --no-filename 'uses:[[:space:]]*actions/checkout@' "${workflow_files[@]}" | wc -l)"
nonpersisting_checkout_count="$(rg -o --no-filename 'persist-credentials:[[:space:]]*false' "${workflow_files[@]}" | wc -l)"
[ "$checkout_count" -eq "$nonpersisting_checkout_count" ] ||
    fail "every checkout must disable persisted GitHub credentials"
job_count="$(rg -o --no-filename '^[[:space:]]{4}runs-on:[[:space:]]*ubuntu-24\.04$' "${workflow_files[@]}" | wc -l)"
timeout_count="$(rg -o --no-filename '^[[:space:]]{4}timeout-minutes:[[:space:]]*[0-9]+$' "${workflow_files[@]}" | wc -l)"
[ "$job_count" -eq "$timeout_count" ] || fail "every CI job must declare a timeout"

for owned_path in \
    '/.github/workflows/' \
    '/.github/dependabot.yml' \
    '/.githooks/' \
    '/Makefile' \
    '/scripts/ci/' \
    '/rust-toolchain.toml' \
    '/tool-versions.env' \
    '/docs/governance/ci-deployment.md' \
    '/docs/governance/supported-platforms.md'; do
    rg -q --fixed-strings "$owned_path @dragginzgame" "$CODEOWNERS" ||
        fail "CODEOWNERS is missing CI authority $owned_path"
done

rg -F 'runs-on: ubuntu-24.04' "$CI" >/dev/null ||
    fail "CI does not declare a job on the canonical ubuntu-24.04 host"
if rg '^[[:space:]]+runs-on:' "$CI" | rg -v '^[[:space:]]+runs-on: ubuntu-24\.04$' >/dev/null; then
    fail "a CI job selects a host outside the canonical ubuntu-24.04 support cell"
fi
rg -F 'bash scripts/ci/install-ic-wasm.sh' "$CI" >/dev/null ||
    fail "CI does not use the checksum-bound ic-wasm installer"
for single_use_tool in \
    'cargo install cargo-get' \
    'cargo install candid-extractor' \
    'bash scripts/ci/install-icp-cli.sh' \
    'bash scripts/ci/install-ic-wasm.sh' \
    'rustup target add'; do
    [ "$(rg -c -F "$single_use_tool" "$CI")" -eq 1 ] ||
        fail "CI must install $single_use_tool exactly once in its owning lane"
done
rg -F 'cargo install cargo-get --version "$CANIC_CARGO_GET_VERSION" --locked' "$CI" >/dev/null ||
    fail "CI does not install the exact pinned cargo-get version"
rg -F 'cargo get --version' "$CI" >/dev/null ||
    fail "CI does not verify the installed cargo-get version"
rg -F 'run: bash scripts/ci/check-release-integrity-contract.sh' "$CI" >/dev/null ||
    fail "release integrity guard is not active in CI"
rg -F 'BIN="$(bash scripts/ci/install-gitleaks.sh)"' "$CI" >/dev/null ||
    fail "CI does not use the checksum-bound Gitleaks installer"
rg -F 'run: bash scripts/ci/run-secret-scan.sh' "$CI" >/dev/null ||
    fail "the dedicated secret scan is not active in CI"
rg -F 'run: bash scripts/ci/check-dependency-risk-inventory.sh' "$CI" >/dev/null ||
    fail "the dependency risk inventory gate is not active in CI"
rg -F 'bash scripts/ci/test-dependency-risk-inventory.sh' "$CI" >/dev/null ||
    fail "the dependency risk rejection tests are not active in CI"
rg -F 'run: bash scripts/ci/check-current-document-semantics.sh' "$CI" >/dev/null ||
    fail "the current document semantics guard is not active in CI"
rg -F '"$SHELLCHECK_BIN" --exclude=SC2001,SC2016' "$CI" >/dev/null ||
    fail "CI does not run the pinned ShellCheck boundary"
ordinary_job="$(sed -n '/^  tests-ordinary:/,/^  tests-pocketic:/p' "$CI")"
rg -F 'cargo install ripgrep --version "$CANIC_RIPGREP_VERSION" --locked --features pcre2' \
    <<<"$ordinary_job" >/dev/null ||
    fail "CI ordinary tests do not install the feature-qualified ripgrep test helper"
rg -F 'rg --version' <<<"$ordinary_job" >/dev/null ||
    fail "CI ordinary tests do not verify the ripgrep test helper"
rg -F 'rg --pcre2-version' <<<"$ordinary_job" >/dev/null ||
    fail "CI ordinary tests do not verify ripgrep PCRE2 support"
pocketic_job="$(sed -n '/^  tests-pocketic:/,/^  release-build:/p' "$CI")"
rg -F 'cargo install ripgrep --version "$CANIC_RIPGREP_VERSION" --locked --features pcre2' \
    <<<"$pocketic_job" >/dev/null ||
    fail "CI PocketIC tests do not install the feature-qualified ripgrep test helper"
rg -F 'rg --version' <<<"$pocketic_job" >/dev/null ||
    fail "CI PocketIC tests do not verify the ripgrep test helper"
validate_recipe="$(sed -n '/^validate:/,/^$/p' "$MAKEFILE")"
required_validate_targets=(
    fmt-check
    check-invariants
    dependency-risk-gate
    gitleaks-scan
    shellcheck
    control-plane-feature-gate
    check
    clippy
    test
)
for validate_target in "${required_validate_targets[@]}"; do
    rg -F "\$(MAKE) --no-print-directory $validate_target" <<<"$validate_recipe" >/dev/null ||
        fail "make validate omits required target $validate_target"
done

invariant_recipe="$(sed -n '/^check-invariants:/,/^$/p' "$MAKEFILE")"
# shellcheck disable=SC2016 # These are literal Make recipe fragments, not shell expansions.
for invariant_command in \
    'bash scripts/ci/run-layering-guards.sh' \
    'bash scripts/ci/check-current-document-semantics.sh' \
    '$(MAKE) --no-print-directory blob-storage-inventory-gate' \
    '$(MAKE) --no-print-directory blob-storage-cashier-inventory-gate' \
    'bash scripts/ci/test-dependency-risk-inventory.sh' \
    'bash scripts/ci/check-release-validation-matrix.sh' \
    'bash scripts/ci/check-release-integrity-contract.sh' \
    'bash scripts/ci/check-audit-method-catalog.sh' \
    'bash scripts/ci/check-recovery-runbooks.sh'; do
    rg -F "$invariant_command" <<<"$invariant_recipe" >/dev/null ||
        fail "make check-invariants omits $invariant_command"
done

declare -A primitive_commands=(
    [build]='cargo build'
    [check]='cargo check'
    [clippy]='cargo clippy'
    [fmt]='cargo fmt'
    [fmt-check]='cargo fmt'
)
for primitive_target in "${!primitive_commands[@]}"; do
    primitive_recipe="$(sed -n "/^$primitive_target:/,/^$/p" "$MAKEFILE")"
    rg -F "${primitive_commands[$primitive_target]}" <<<"$primitive_recipe" >/dev/null ||
        fail "make $primitive_target omits its named Cargo operation"
    if rg '\$\(MAKE\)|scripts/' <<<"$primitive_recipe" >/dev/null; then
        fail "make $primitive_target delegates hidden repository work"
    fi
    case "$primitive_target" in
        build | check | clippy)
            rg -F -- '--locked' <<<"$primitive_recipe" >/dev/null ||
                fail "make $primitive_target does not freeze Cargo.lock"
            ;;
    esac
done
rg -F 'cargo test --locked "$@"' "$WORKSPACE_TEST_RUNNER" >/dev/null ||
    fail "workspace test execution does not freeze Cargo.lock"

[ -x "$PRE_COMMIT_HOOK" ] || fail "formatting pre-commit hook is not executable"
hook_file_count="$(find "$ROOT/.githooks" -maxdepth 1 -type f | wc -l)"
[ "$hook_file_count" -eq 1 ] || fail "repository must own exactly one Git hook"
rg -F 'make fmt' "$PRE_COMMIT_HOOK" >/dev/null ||
    fail "pre-commit hook does not run the complete formatter"
rg -F 'git diff --cached --name-only -z --diff-filter=ACMR' "$PRE_COMMIT_HOOK" >/dev/null ||
    fail "pre-commit hook does not reject partially staged formatting inputs"
rg -F 'git diff --binary --no-ext-diff' "$PRE_COMMIT_HOOK" >/dev/null ||
    fail "pre-commit hook does not snapshot tracked working-tree content"
rg -F 'cmp -s "$before" "$after"' "$PRE_COMMIT_HOOK" >/dev/null ||
    fail "pre-commit hook does not reject formatter mutations"
if rg -n 'git[[:space:]]+add|make[[:space:]]+(fmt-check|validate|test|clippy|build)|cargo[[:space:]]+(test|clippy|build)|git[[:space:]]+(commit|push)' \
    "$PRE_COMMIT_HOOK" >/dev/null; then
    fail "pre-commit hook exceeds its formatting-only boundary"
fi
rg -F 'core.hooksPath .githooks' "$GIT_HOOK_INSTALLER" >/dev/null ||
    fail "Git hook installer does not configure the repository hook path"
rg -F 'scripts/dev/install-git-hooks.sh' "$DEV_INSTALL" >/dev/null ||
    fail "maintainer toolchain setup does not install the formatting hook"
install_hooks_recipe="$(sed -n '/^install-hooks:/,/^$/p' "$MAKEFILE")"
rg -F 'bash scripts/dev/install-git-hooks.sh' <<<"$install_hooks_recipe" >/dev/null ||
    fail "make install-hooks does not use the canonical hook installer"
bash "$WORKSPACE_TEST_INVENTORY_GATE" >/dev/null ||
    fail "the workspace integration-test inventory is incomplete or invalid"
rg -F 'bash scripts/ci/check-workspace-test-inventory.sh' "$WORKSPACE_TEST_RUNNER" >/dev/null ||
    fail "the workspace test runner does not enforce its integration-test inventory"
rg -F 'POCKET_IC_BIN="$(bash scripts/ci/install-pocketic.sh)"' "$WORKSPACE_TEST_RUNNER" >/dev/null ||
    fail "the full workspace test runner does not resolve one explicit PocketIC server"
rg -F 'CANIC_POCKET_IC_CACHE_DIR' "$ROOT/scripts/ci/install-pocketic.sh" >/dev/null ||
    fail "the PocketIC installer does not expose its persistent local cache boundary"
if rg -F '${TMPDIR' "$ROOT/scripts/ci/install-pocketic.sh" >/dev/null; then
    fail "the PocketIC server binary must not live in disposable test scratch"
fi
if rg --multiline 'test(-wasm)?:[^\n]*(\\\n[^\n]*)?workspace-test-inventory-gate' "$MAKEFILE" >/dev/null; then
    fail "the public test targets duplicate the workspace runner inventory guard"
fi
rg -F 'run: cargo build -p canic --examples --locked' "$CI" >/dev/null ||
    fail "CI omits the default example build"
if rg '^[[:space:]]+tags:' "$CI" >/dev/null; then
    fail "primary CI must not create a second tag-only release signal"
fi
rg -F "startsWith(github.event.head_commit.message, 'Release ')" "$CI" >/dev/null ||
    fail "the release workspace build is not bound to a main release commit"
rg -F 'run: cargo build --release --workspace --locked' "$CI" >/dev/null ||
    fail "CI omits the release-profile workspace build"
rg -F 'run_serial_pocketic_test' "$WORKSPACE_TEST_RUNNER" >/dev/null ||
    fail "the workspace test runner does not isolate serial PocketIC execution"
CANIC_TEST_PLAN_ONLY=1 bash "$WORKSPACE_TEST_RUNNER" fast >/dev/null ||
    fail "the fast workspace test plan cannot be resolved"
CANIC_TEST_PLAN_ONLY=1 bash "$WORKSPACE_TEST_RUNNER" full >/dev/null ||
    fail "the full workspace test plan cannot be resolved"
CANIC_TEST_PLAN_ONLY=1 bash "$WORKSPACE_TEST_RUNNER" ordinary >/dev/null ||
    fail "the ordinary workspace test plan cannot be resolved"
CANIC_TEST_PLAN_ONLY=1 bash "$WORKSPACE_TEST_RUNNER" pocketic >/dev/null ||
    fail "the PocketIC workspace test plan cannot be resolved"
for mode in patch minor major; do
    mode_recipe="$(sed -n "/^$mode:/,/^$/p" "$MAKEFILE")"
    if rg -F '$(MAKE) --no-print-directory fmt' <<<"$mode_recipe" >/dev/null; then
        fail "the $mode version target must not mutate formatting"
    fi
    rg -F '$(MAKE) --no-print-directory validate' <<<"$mode_recipe" >/dev/null ||
        fail "the $mode version target does not run the explicit validation workflow"
    rg -F "CANIC_RELEASE_VALIDATED=1 scripts/ci/bump-version.sh $mode" <<<"$mode_recipe" >/dev/null ||
        fail "the $mode version target does not bind mutation to completed validation"
done
patch_recipe="$(sed -n '/^patch:/,/^$/p' "$MAKEFILE")"
rg -F '$(MAKE) --no-print-directory release-cadence' <<<"$patch_recipe" >/dev/null ||
    fail "the patch release flow omits its read-only cadence advisory"
cadence_output="$(bash "$RELEASE_CADENCE")"
rg -F 'guideline: no more than 12 releases per minor' <<<"$cadence_output" >/dev/null ||
    fail "the release cadence tool does not report the governed release-count guideline"
rg -F 'next release ordinal:' <<<"$cadence_output" >/dev/null ||
    fail "the release cadence tool does not report the next release ordinal"
rg -F 'CANIC_RELEASE_VALIDATED' "$BUMP_VERSION" >/dev/null ||
    fail "direct release version mutation is not guarded by completed validation"
rg -F 'cargo metadata --locked --offline --format-version 1 --no-deps' "$RELEASE_CANDIDATE" >/dev/null ||
    fail "post-bump release candidate does not verify locked offline metadata"
for version_consumer in \
    "$MAKEFILE" \
    "$BUMP_VERSION" \
    "$CONFIRM_VERSION_BUMP" \
    "$RELEASE_CANDIDATE" \
    "$RELEASE_CADENCE" \
    "$PUBLISH_WORKSPACE" \
    "$RELEASE_PUSH_READY" \
    "$RELEASE_PUSH"; do
    rg -F 'read-workspace-version.sh' "$version_consumer" >/dev/null ||
        fail "workspace-version consumer bypasses the canonical cargo-get reader: $version_consumer"
done
release_commit_recipe="$(sed -n '/^release-commit:/,/^$/p' "$MAKEFILE")"
rg -F '$(MAKE) --no-print-directory release-candidate' <<<"$release_commit_recipe" >/dev/null ||
    fail "release commit does not verify the exact post-bump candidate"
for release_target in patch minor major; do
    release_recipe="$(sed -n "/^$release_target:/,/^$/p" "$MAKEFILE")"
    clean_count="$(rg -c '\$\(MAKE\) ensure-clean' <<<"$release_recipe")"
    [ "$clean_count" -eq 2 ] ||
        fail "make $release_target must verify cleanliness before and after validation"
done
release_push_recipe="$(sed -n '/^release-push:/,/^$/p' "$MAKEFILE")"
expected_release_push_recipe=$'release-push:\n\t@bash scripts/ci/check-release-push-ready.sh\n\t@CANIC_RELEASE_PUSH_READY=1 bash scripts/ci/push-release.sh'
[ "$release_push_recipe" = "$expected_release_push_recipe" ] ||
    fail "release push is not limited to readiness and the atomic network update"
for release_push_script in "$RELEASE_PUSH_READY" "$RELEASE_PUSH"; do
    rg -F 'read-workspace-version.sh' "$release_push_script" >/dev/null ||
        fail "release push does not use the canonical workspace-version reader"
    rg -F -- '--committed' "$release_push_script" >/dev/null ||
        fail "release push does not derive its version from committed HEAD"
done
rg -F 'cargo get --entry "$entry" workspace.package.version' "$VERSION_READER" >/dev/null ||
    fail "workspace-version reader does not use cargo-get"
rg -F 'git show HEAD:Cargo.toml' "$VERSION_READER" >/dev/null ||
    fail "workspace-version reader cannot inspect committed HEAD"
if rg -F 'git status --porcelain' "$RELEASE_PUSH_READY" >/dev/null; then
    fail "release push still rejects unrelated local worktree or index changes"
fi
rg -F 'cargo clean' "$RELEASE_CLEANUP" >/dev/null ||
    fail "release cleanup does not clear Cargo build artifacts"
rg -F 'MAX_CARGO_CLEAN_ATTEMPTS=2' "$RELEASE_CLEANUP" >/dev/null ||
    fail "release cleanup does not bound its Cargo cleanup retry"
rg -F 'CANIC_TEST_SCRATCH' "$RELEASE_CLEANUP" >/dev/null ||
    fail "release cleanup does not require exact invocation-owned test scratch"
rg -F 'bash "$POCKET_IC_STOPPER"' "$RELEASE_CLEANUP" >/dev/null ||
    fail "release cleanup can remove scratch before its PocketIC server exits"
rg -F 'is_owned_port_file' "$POCKET_IC_STOPPER" >/dev/null ||
    fail "PocketIC cleanup does not bind termination to an owned port file"
rg -F 'kill -KILL "$pid"' "$POCKET_IC_STOPPER" >/dev/null ||
    fail "PocketIC cleanup does not stop its detached server before scratch removal"
rg -F 'test-runtime\.[[:alnum:]]{6}' "$RELEASE_CLEANUP" >/dev/null ||
    fail "release cleanup does not validate the private scratch basename"
rg -F 'mktemp -d "$TEST_SCRATCH_PARENT/test-runtime.XXXXXX"' "$TEST_SCRATCH_RUNNER" >/dev/null ||
    fail "test scratch runner does not allocate one private repository directory"
rg -F 'CANIC_TEST_SCRATCH="$TEST_SCRATCH"' "$TEST_SCRATCH_RUNNER" >/dev/null ||
    fail "test scratch runner does not pass exact cleanup ownership"
test_scratch_runner_count="$(rg -c 'bash scripts/ci/run-with-test-scratch\.sh' "$MAKEFILE")"
[ "$test_scratch_runner_count" -gt 0 ] ||
    fail "temporary-file test targets do not use private scratch ownership"
if rg -F 'TEST_TMPDIR' "$MAKEFILE" >/dev/null; then
    fail "Make retains the superseded shared test-scratch variable"
fi
if rg -F 'TEST_SCRATCH="$TEST_SCRATCH_PARENT/test-runtime"' "$RELEASE_CLEANUP" >/dev/null; then
    fail "release cleanup retains the shared test-scratch deletion path"
fi
rg -F 'git push --no-follow-tags --atomic origin' "$RELEASE_PUSH" >/dev/null ||
    fail "release push does not require one atomic remote update"
rg -F '"HEAD:refs/heads/$branch"' "$RELEASE_PUSH" >/dev/null ||
    fail "release push does not name the exact branch ref"
rg -F '"refs/tags/$tag:refs/tags/$tag"' "$RELEASE_PUSH" >/dev/null ||
    fail "release push does not name the exact release tag ref"
if bash "$RELEASE_PUSH" >/dev/null 2>&1; then
    fail "release push helper accepted direct unverified invocation"
fi
if rg -F 'rm -rf -- /tmp' "$RELEASE_CLEANUP" >/dev/null; then
    fail "release cleanup may not delete an unscoped global temporary path"
fi
rg -F -- '--redact=100' "$SECRET_SCAN" >/dev/null ||
    fail "the dedicated secret scan does not redact findings"
rg -F '"$GITLEAKS_BIN" git' "$SECRET_SCAN" >/dev/null ||
    fail "the dedicated scanner does not inspect Git history"
rg -F -- '--gitleaks-ignore-path "$ROOT_DIR/.gitleaksignore"' "$SECRET_SCAN" >/dev/null ||
    fail "the dedicated secret scan does not select the reviewed fingerprint file"
rg -F 'Gitleaks configuration overrides are not allowed' "$SECRET_SCAN" >/dev/null ||
    fail "the dedicated secret scan does not reject external rule configuration"
rg -F 'repository .gitleaks.toml overrides are not allowed' "$SECRET_SCAN" >/dev/null ||
    fail "the dedicated secret scan does not reject repository rule configuration"
rg -F -- '--is-shallow-repository' "$SECRET_SCAN" >/dev/null ||
    fail "the dedicated secret scan does not reject incomplete Git history"
rg -F '[ "$version_output" != "$CANIC_GITLEAKS_VERSION" ]' "$SECRET_SCAN" >/dev/null ||
    fail "the dedicated secret scan does not require the exact Gitleaks version"
rg -F '[ "$version_output" != "$VERSION" ]' "$ROOT/scripts/ci/install-gitleaks.sh" >/dev/null ||
    fail "the Gitleaks installer does not require the exact reported version"
rg -F 'cargo update --workspace --offline' "$BUMP_VERSION" >/dev/null ||
    fail "the release bump does not preserve locked external dependency identities"

gitleaks_ignore_count=0
while IFS= read -r fingerprint; do
    case "$fingerprint" in
    '' | \#*) continue ;;
    esac
    [[ "$fingerprint" =~ ^[0-9a-f]{40}:.+:[a-z0-9-]+:[0-9]+$ ]] ||
        fail "invalid Gitleaks fingerprint entry"
    gitleaks_ignore_count=$((gitleaks_ignore_count + 1))
done <"$GITLEAKS_IGNORE"
[ "$gitleaks_ignore_count" -gt 0 ] || fail "no reviewed Gitleaks fingerprints were found"

while IFS= read -r install_command; do
    if [[ "$install_command" != *"--version"* ]]; then
        fail "CI Cargo helper install lacks an exact version: $install_command"
    fi
done < <(rg '^[[:space:]]*cargo install ' "$CI")

# shellcheck source=/dev/null
source "$TOOLS"

rust_toolchain="$(
    sed -n 's/^channel = "\([^"]*\)"$/\1/p' "$RUST_TOOLCHAIN"
)"
[ -n "$rust_toolchain" ] || fail "rust-toolchain.toml does not declare a channel"
rg -F "CANIC_INTERNAL_TOOLCHAIN: $rust_toolchain" "$CI" >/dev/null ||
    fail "CI internal Rust does not match rust-toolchain.toml"
rg -F "CANIC_RUST_TOOLCHAIN=\"\${CANIC_RUST_TOOLCHAIN:-$rust_toolchain}\"" "$DEV_INSTALL" >/dev/null ||
    fail "developer bootstrap Rust does not match rust-toolchain.toml"
rg -F "internal%20rust-$rust_toolchain-orange.svg" "$README" >/dev/null ||
    fail "README internal Rust badge does not match rust-toolchain.toml"
rg -F 'pins internal Rust `'"$rust_toolchain"'`' "$README" >/dev/null ||
    fail "README internal Rust text does not match rust-toolchain.toml"

icp_cli_required="$({
    sed -n 's/^pub(super) const REQUIRED_ICP_CLI_VERSION: &str = "\([^"]*\)";$/\1/p' "$ICP_MODEL"
} || true)"
[[ "$icp_cli_required" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] ||
    fail "host ICP CLI minimum is not one exact semantic version"
IFS=. read -r required_icp_major required_icp_minor required_icp_patch <<<"$icp_cli_required"
IFS=. read -r pinned_icp_major pinned_icp_minor pinned_icp_patch <<<"$CANIC_ICP_CLI_VERSION"
[ "$pinned_icp_major" -eq "$required_icp_major" ] ||
    fail "pinned ICP CLI major does not match the supported host major"
if [ "$pinned_icp_minor" -lt "$required_icp_minor" ] ||
    { [ "$pinned_icp_minor" -eq "$required_icp_minor" ] &&
        [ "$pinned_icp_patch" -lt "$required_icp_patch" ]; }; then
    fail "pinned ICP CLI is older than the supported host minimum"
fi
next_icp_major=$((required_icp_major + 1))
rg -F "ICP_CLI_SUPPORTED_VERSION_RANGE: &str = \">=$icp_cli_required, <$next_icp_major.0.0\"" "$ICP_MODEL" >/dev/null ||
    fail "host ICP CLI range does not match its independent minimum"
rg -F "echo \"icp-cli $icp_cli_required\"" "$ICP_PROOF" >/dev/null ||
    fail "CLI proof fixture does not report the supported ICP CLI floor"
rg -F 'maintainer toolchain currently pins `'"$CANIC_ICP_CLI_VERSION"'`' "$INSTALLING" >/dev/null ||
    fail "installation guidance does not report the pinned ICP CLI version"
rg -F 'bash scripts/dev/update-icp-cli-pin.sh' "$MAKEFILE" >/dev/null ||
    fail "make update-dev does not refresh the ICP CLI pin"

update_dev_recipe="$(sed -n '/^update-dev:/,/^$/p' "$MAKEFILE")"
rg -F 'bash scripts/dev/install_dev.sh --ensure-ripgrep' <<<"$update_dev_recipe" >/dev/null ||
    fail "make update-dev does not install the feature-qualified ripgrep tool"
rg -F '"$(CARGO_INSTALL_BIN_DIR)/rg" --pcre2-version' <<<"$update_dev_recipe" >/dev/null ||
    fail "make update-dev does not verify ripgrep PCRE2 support"
rg -F 'bash scripts/ci/check-dependency-risk-inventory.sh' <<<"$update_dev_recipe" >/dev/null ||
    fail "make update-dev does not use the isolated dependency risk gate"
if rg -F 'cargo audit' <<<"$update_dev_recipe" >/dev/null; then
    fail "make update-dev must not use cargo-audit's mutable shared database"
fi
rg -F 'cargo_toolchain install --quiet --locked --force --features pcre2' "$DEV_INSTALL" >/dev/null ||
    fail "developer ripgrep installation does not enable required PCRE2 support"

mapfile -t version_vars < <(
    sed -n 's/^export \(CANIC_[A-Z0-9_]*_VERSION\)=.*/\1/p' "$TOOLS"
)
[ "${#version_vars[@]}" -gt 0 ] || fail "no exact tool-version pins were found"
declare -A validated_version_vars=()
for variable in "${version_vars[@]}"; do
    value="${!variable:-}"
    [[ "$value" =~ ^[0-9]+\.[0-9]+\.[0-9]+([+-][0-9A-Za-z.-]+)?$ ]] ||
        fail "$variable is not an exact semantic version"
    validated_version_vars["$variable"]=1
done

sha256_count=0
declare -A validated_checksum_vars=()
while IFS='=' read -r variable digest; do
    case "$variable" in
    export\ CANIC_*_SHA256*)
        [[ "$digest" =~ ^[0-9a-f]{64}$ ]] || fail "invalid SHA-256 pin: $variable"
        validated_checksum_vars["${variable#export }"]=1
        sha256_count=$((sha256_count + 1))
        ;;
    export\ CANIC_*_SHA512*)
        [[ "$digest" =~ ^[0-9a-f]{128}$ ]] || fail "invalid SHA-512 pin: $variable"
        validated_checksum_vars["${variable#export }"]=1
        ;;
    esac
done <"$TOOLS"

[ "$sha256_count" -gt 0 ] || fail "no SHA-256 pins were found"

for installer in "${installers[@]}"; do
    rg -F 'verify-file-checksum.sh' "$installer" >/dev/null ||
        fail "installer does not verify downloaded content: $installer"
    rg -F -- "--proto-redir '=https'" "$installer" >/dev/null ||
        fail "installer does not constrain redirect protocols: $installer"
    rg '\$CANIC_[A-Z0-9_]*_VERSION' "$installer" >/dev/null ||
        fail "installer does not use a repository version pin: $installer"
    rg '\$CANIC_[A-Z0-9_]+_SHA(256|512)_[A-Z0-9_]+' "$installer" >/dev/null ||
        fail "installer does not use a repository checksum pin: $installer"
done

mapfile -t referenced_version_vars < <(
    rg -o --no-filename '\$CANIC_[A-Z0-9_]*_VERSION' \
        "${installers[@]}" "$ICP_REQUIRE" | sed 's/^\$//' | sort -u
)
[ "${#referenced_version_vars[@]}" -gt 0 ] ||
    fail "tool consumers do not reference repository version pins"
for variable in "${referenced_version_vars[@]}"; do
    [ -n "${validated_version_vars[$variable]:-}" ] ||
        fail "tool consumer references an unvalidated version pin: $variable"
done

mapfile -t referenced_checksum_vars < <(
    rg -o --no-filename '\$CANIC_[A-Z0-9_]+_SHA(256|512)_[A-Z0-9_]+' \
        "${installers[@]}" | sed 's/^\$//' | sort -u
)
[ "${#referenced_checksum_vars[@]}" -gt 0 ] ||
    fail "installers do not reference repository checksum pins"
for variable in "${referenced_checksum_vars[@]}"; do
    [ -n "${validated_checksum_vars[$variable]:-}" ] ||
        fail "installer references an unvalidated checksum pin: $variable"
done

caller_override_result="$(
    CANIC_ICP_CLI_VERSION=0.0.0 CANIC_IC_WASM_VERSION=0.0.0 \
        bash -c 'source "$1"; printf "%s %s\n" "$CANIC_ICP_CLI_VERSION" "$CANIC_IC_WASM_VERSION"' \
        _ "$ICP_REQUIRE"
)"
[ "$caller_override_result" = "$CANIC_ICP_CLI_VERSION $CANIC_IC_WASM_VERSION" ] ||
    fail "caller values can override the canonical IC tool pins"

if wrong_ic_wasm_output="$(
    bash -c '
        source "$1"
        icp() { printf "icp-cli %s\n" "$CANIC_ICP_CLI_VERSION"; }
        ic-wasm() { printf "ic-wasm 0.0.0\n"; }
        require_icp_tools
    ' _ "$ICP_REQUIRE" 2>&1
)"; then
    fail "the IC prerequisite check accepted an unpinned ic-wasm version"
fi
[[ "$wrong_ic_wasm_output" == *"unsupported ic-wasm version for Canic CI"* ]] ||
    fail "the IC prerequisite check did not preserve its version-mismatch cause"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

release_cleanup_fixture="$tmp_dir/release-cleanup"
release_cleanup_bin="$release_cleanup_fixture/bin"
foreign_test_scratch="$release_cleanup_fixture/.tmp/test-runtime.FOREIGN"
mkdir -p \
    "$release_cleanup_fixture/scripts/ci" \
    "$foreign_test_scratch" \
    "$release_cleanup_fixture/target" \
    "$release_cleanup_bin"
touch "$foreign_test_scratch/live-owner"
cp "$RELEASE_CLEANUP" "$TEST_SCRATCH_RUNNER" "$POCKET_IC_STOPPER" \
    "$release_cleanup_fixture/scripts/ci/"
# shellcheck disable=SC2016 # Preserve expansion for the generated fixture.
printf '%s\n' \
    '#!/usr/bin/env bash' \
    '[ "${1:-}" = "clean" ] || exit 2' \
    'attempt_file="$PWD/cargo-clean-attempts"' \
    'attempt=0' \
    '[ ! -f "$attempt_file" ] || read -r attempt <"$attempt_file"' \
    'attempt=$((attempt + 1))' \
    'printf "%s\n" "$attempt" >"$attempt_file"' \
    '[ "$attempt" -gt "${FAKE_CARGO_FAILURES:-0}" ] || exit "${FAKE_CARGO_STATUS:-19}"' \
    'rm -rf -- "$PWD/target"' >"$release_cleanup_bin/cargo"
chmod +x "$release_cleanup_bin/cargo"

assert_owned_test_scratch_cleaned() {
    local scratch name

    scratch="$(cat "$release_cleanup_fixture/test-tmpdir")"
    name="${scratch##*/}"
    [ "${scratch%/*}" = "$release_cleanup_fixture/.tmp" ] &&
        [[ "$name" =~ ^test-runtime\.[[:alnum:]]{6}$ ]] ||
        fail "test runner did not select one private repository scratch directory"
    [ ! -e "$scratch" ] ||
        fail "test runner retained its invocation-owned test scratch"
    [ -f "$foreign_test_scratch/live-owner" ] ||
        fail "test cleanup deleted another invocation's test scratch"
}

PATH="$release_cleanup_bin:$PATH" \
    bash "$release_cleanup_fixture/scripts/ci/run-with-test-scratch.sh" \
    bash -c 'printf "%s\n" "$TMPDIR" >"$1"' _ "$release_cleanup_fixture/test-tmpdir"
assert_owned_test_scratch_cleaned

rm -f "$release_cleanup_fixture/cargo-clean-attempts"
mkdir -p "$release_cleanup_fixture/target"
FAKE_CARGO_FAILURES=1 PATH="$release_cleanup_bin:$PATH" \
    bash "$release_cleanup_fixture/scripts/ci/cleanup-release-artifacts.sh"
[ "$(cat "$release_cleanup_fixture/cargo-clean-attempts")" -eq 2 ] ||
    fail "release cleanup did not retry one transient Cargo failure exactly once"
[ ! -e "$release_cleanup_fixture/target" ] ||
    fail "retried release cleanup retained Cargo artifacts"
[ -f "$foreign_test_scratch/live-owner" ] ||
    fail "explicit Cargo cleanup deleted another invocation's test scratch"

rm -f "$release_cleanup_fixture/cargo-clean-attempts"
mkdir -p "$release_cleanup_fixture/target"
if FAKE_CARGO_FAILURES=2 FAKE_CARGO_STATUS=19 PATH="$release_cleanup_bin:$PATH" \
    bash "$release_cleanup_fixture/scripts/ci/cleanup-release-artifacts.sh"; then
    fail "explicit cleanup accepted a failed Cargo cleanup"
else
    release_cleanup_status=$?
fi
[ "$release_cleanup_status" -eq 1 ] ||
    fail "explicit cleanup did not preserve the Cargo cleanup failure"
[ "$(cat "$release_cleanup_fixture/cargo-clean-attempts")" -eq 2 ] ||
    fail "release cleanup exceeded its bounded Cargo retry"
[ -e "$release_cleanup_fixture/target" ] ||
    fail "failed fake Cargo cleanup unexpectedly removed its target fixture"

borrowed_test_scratch="$release_cleanup_fixture/.tmp/test-runtime.BORROW"
mkdir -p "$borrowed_test_scratch"
CANIC_TEST_SCRATCH="$borrowed_test_scratch" \
    bash "$release_cleanup_fixture/scripts/ci/run-with-test-scratch.sh" \
    bash -c '[ "$TMPDIR" = "$CANIC_TEST_SCRATCH" ]'
[ -d "$borrowed_test_scratch" ] ||
    fail "nested test runner deleted scratch owned by its caller"
CANIC_TEST_SCRATCH="$borrowed_test_scratch" \
    bash "$release_cleanup_fixture/scripts/ci/cleanup-release-artifacts.sh" --scratch-only
[ ! -e "$borrowed_test_scratch" ] ||
    fail "explicit scratch owner could not clear its private directory"

touch "$release_cleanup_fixture/.tmp/path-escape-sentinel"
if CANIC_TEST_SCRATCH="$release_cleanup_fixture/.tmp/test-runtime.BAD123/.." \
    bash "$release_cleanup_fixture/scripts/ci/cleanup-release-artifacts.sh" --scratch-only; then
    fail "release cleanup accepted a non-direct scratch target"
fi
[ -f "$release_cleanup_fixture/.tmp/path-escape-sentinel" ] ||
    fail "release cleanup followed an unowned scratch path"

ln -s "$foreign_test_scratch" "$release_cleanup_fixture/.tmp/test-runtime.LINK12"
if CANIC_TEST_SCRATCH="$release_cleanup_fixture/.tmp/test-runtime.LINK12" \
    bash "$release_cleanup_fixture/scripts/ci/cleanup-release-artifacts.sh" --scratch-only; then
    fail "release cleanup accepted a symlinked scratch target"
fi
[ -f "$foreign_test_scratch/live-owner" ] ||
    fail "release cleanup followed a symlink into another invocation's scratch"

owned_server_scratch="$release_cleanup_fixture/.tmp/test-runtime.SERVER"
owned_server_port="$owned_server_scratch/pocket_ic_12345.port"
foreign_server_port="$foreign_test_scratch/pocket_ic_67890.port"
mkdir -p "$owned_server_scratch"
touch "$owned_server_port" "$foreign_server_port"
bash -c 'exec -a pocket-ic bash -c "while :; do sleep 1; done" -- --port-file "$1"' \
    _ "$owned_server_port" &
owned_server_pid=$!
bash -c 'exec -a pocket-ic bash -c "while :; do sleep 1; done" -- --port-file "$1"' \
    _ "$foreign_server_port" &
foreign_server_pid=$!
sleep 0.1
CANIC_TEST_SCRATCH="$owned_server_scratch" \
    bash "$release_cleanup_fixture/scripts/ci/cleanup-release-artifacts.sh" --scratch-only
wait "$owned_server_pid" 2>/dev/null || :
if kill -0 "$owned_server_pid" 2>/dev/null; then
    fail "release cleanup retained its invocation-owned PocketIC server"
fi
kill -0 "$foreign_server_pid" 2>/dev/null ||
    fail "release cleanup stopped another invocation's PocketIC server"
[ ! -e "$owned_server_scratch" ] ||
    fail "release cleanup retained scratch after its PocketIC server stopped"
kill -KILL "$foreign_server_pid" 2>/dev/null || :
wait "$foreign_server_pid" 2>/dev/null || :

release_push_fixture="$tmp_dir/release-push"
release_push_bin="$release_push_fixture/bin"
mkdir -p "$release_push_fixture/scripts/ci" "$release_push_bin"
cp "$RELEASE_PUSH" "$VERSION_READER" "$release_push_fixture/scripts/ci/"
printf '%s\n' \
    '[workspace.package]' \
    'version = "9.9.9"' >"$release_push_fixture/Cargo.toml"
printf '%s\n' \
    '[workspace.package]' \
    'version = "0.101.10"' >"$release_push_fixture/committed-Cargo.toml"
# shellcheck disable=SC2016 # Preserve argument handling for the generated fixture.
printf '%s\n' \
    '#!/usr/bin/env bash' \
    'case "${1:-}" in' \
    'symbolic-ref) printf "main\n" ;;' \
    'show) cat "$PWD/committed-Cargo.toml" ;;' \
    'push) printf "%s\n" "$@" >"$PWD/push-arguments" ;;' \
    '*) exit 2 ;;' \
    'esac' >"$release_push_bin/git"
chmod +x "$release_push_bin/git"
CANIC_RELEASE_PUSH_READY=1 PATH="$release_push_bin:$PATH" \
    bash "$release_push_fixture/scripts/ci/push-release.sh"
expected_push_arguments=$'push\n--no-follow-tags\n--atomic\norigin\nHEAD:refs/heads/main\nrefs/tags/v0.101.10:refs/tags/v0.101.10'
[ "$(cat "$release_push_fixture/push-arguments")" = "$expected_push_arguments" ] ||
    fail "release push did not send the exact branch and tag refs atomically"

bash "$TAG_DELETE_TEST" >/dev/null ||
    fail "historical-tag deletion fixture failed"

fake_gitleaks="$tmp_dir/gitleaks"
# shellcheck disable=SC2016 # Preserve variable expansion for the generated fixture.
printf '%s\n' \
    '#!/usr/bin/env bash' \
    'case "${1:-}" in' \
    'version)' \
    '    [ "${FAKE_GITLEAKS_VERSION_FAIL:-0}" != "1" ] || exit 1' \
    '    printf "%s\\n" "${FAKE_GITLEAKS_VERSION:-}"' \
    '    ;;' \
    'git) exit 0 ;;' \
    '*) exit 2 ;;' \
    'esac' >"$fake_gitleaks"
chmod +x "$fake_gitleaks"

if unavailable_gitleaks_output="$(
    FAKE_GITLEAKS_VERSION_FAIL=1 GITLEAKS_BIN="$fake_gitleaks" bash "$SECRET_SCAN" 2>&1
)"; then
    fail "the secret scan accepted unavailable Gitleaks version output"
fi
[[ "$unavailable_gitleaks_output" == *"unable to read the gitleaks version"* ]] ||
    fail "the secret scan did not preserve its unavailable-version cause"

if near_gitleaks_output="$(
    FAKE_GITLEAKS_VERSION="${CANIC_GITLEAKS_VERSION}0" \
        GITLEAKS_BIN="$fake_gitleaks" bash "$SECRET_SCAN" 2>&1
)"; then
    fail "the secret scan accepted a near-match Gitleaks version"
fi
[[ "$near_gitleaks_output" == *"gitleaks version mismatch"* ]] ||
    fail "the secret scan did not preserve its version-mismatch cause"

for config_variable in GITLEAKS_CONFIG GITLEAKS_CONFIG_TOML; do
    if config_override_output="$(
        env "$config_variable=review-override" \
            FAKE_GITLEAKS_VERSION="$CANIC_GITLEAKS_VERSION" \
            GITLEAKS_BIN="$fake_gitleaks" bash "$SECRET_SCAN" 2>&1
    )"; then
        fail "the secret scan accepted $config_variable"
    fi
    [[ "$config_override_output" == *"configuration overrides are not allowed"* ]] ||
        fail "the secret scan did not preserve its configuration-override cause"
done

fake_bin="$tmp_dir/bin"
mkdir -p "$fake_bin"
# shellcheck disable=SC2016 # Preserve argument handling for the generated fixture.
printf '%s\n' \
    '#!/usr/bin/env bash' \
    'last=""' \
    'for argument in "$@"; do last="$argument"; done' \
    'case "$last" in' \
    '--is-inside-work-tree) exit 0 ;;' \
    '--is-shallow-repository) printf "true\\n" ;;' \
    '*) exit 2 ;;' \
    'esac' >"$fake_bin/git"
chmod +x "$fake_bin/git"
if shallow_history_output="$(
    PATH="$fake_bin:$PATH" \
        FAKE_GITLEAKS_VERSION="$CANIC_GITLEAKS_VERSION" \
        GITLEAKS_BIN="$fake_gitleaks" bash "$SECRET_SCAN" 2>&1
)"; then
    fail "the secret scan accepted incomplete Git history"
fi
[[ "$shallow_history_output" == *"complete repository history is unavailable in a shallow clone"* ]] ||
    fail "the secret scan did not preserve its shallow-history cause"

if rg -n 'curl[^|]*\|' "${installers[@]}" "$DEV_INSTALL" "$ICP_UPDATE" >/dev/null; then
    fail "active installer pipes an unverified download into execution"
fi

rg -F 'runs-on: ubuntu-24.04' "$CI" >/dev/null ||
    fail "CI does not select the canonical supported host"
rg -F 'Ubuntu 24.04, x86_64' "$MATRIX" >/dev/null ||
    fail "supported host matrix is missing the CI host"
rg -F '`x86_64-unknown-linux-gnu`' "$MATRIX" >/dev/null ||
    fail "supported host matrix is missing the native target"
rg -F '`wasm32-unknown-unknown`' "$MATRIX" >/dev/null ||
    fail "supported host matrix is missing the canister target"
rg -F 'Install-Capable But Not Release-Supported' "$MATRIX" >/dev/null ||
    fail "supported host matrix does not distinguish installer branches"

printf 'canic-release-integrity\n' >"$tmp_dir/input"
bash "$VERIFY" sha256 \
    ef57c7341ccbad50924ce5ffe7d2069b1106acac606f1f8ebd92b5b0a47067df \
    "$tmp_dir/input"
if bash "$VERIFY" sha256 \
    0000000000000000000000000000000000000000000000000000000000000000 \
    "$tmp_dir/input" >"$tmp_dir/rejection.stdout" 2>"$tmp_dir/rejection.stderr"; then
    fail "checksum mismatch was accepted"
fi
rg -F 'sha256 checksum mismatch' "$tmp_dir/rejection.stderr" >/dev/null ||
    fail "checksum mismatch did not preserve its deterministic cause"

bash -n "$VERIFY" "${installers[@]}" "$SECRET_SCAN" "$POCKET_IC_ALIGNMENT" "$RELEASE_CANDIDATE" "$VERSION_READER" "$DEV_INSTALL" "$ICP_UPDATE"
bash "$POCKET_IC_ALIGNMENT" >/dev/null

echo "release integrity contract guard passed ($external_action_count immutable Actions)"
