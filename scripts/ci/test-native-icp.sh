#!/usr/bin/env bash
# Subshell isolation and an unchanged parent PATH are deliberate assertions.
# shellcheck disable=SC2030,SC2031
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=./scripts/ci/native-icp-lib.sh
source "$ROOT/scripts/ci/native-icp-lib.sh"
required_version="$(
    # shellcheck source=/dev/null
    source "$ROOT/tool-versions.env"
    printf '%s' "$CANIC_ICP_CLI_VERSION"
)"
fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT
original_path="$PATH"
mkdir -p "$fixture/cargo/bin" "$fixture/launcher" "$fixture/alternate path" "$fixture/scratch"
cat > "$fixture/native.rs" <<'RUST'
fn main() {
    assert_eq!(std::env::args().nth(1).as_deref(), Some("--version"));
    println!("icp {}", std::env::var("NATIVE_ICP_FIXTURE_VERSION").unwrap());
}
RUST
rustc --edition 2024 --crate-name native_icp_fixture "$fixture/native.rs" -o "$fixture/cargo/bin/icp"
cp "$fixture/cargo/bin/icp" "$fixture/alternate path/icp"
cat > "$fixture/launcher/icp" <<'SH'
#!/bin/sh
printf 'launcher must not run\n' >&2
exit 91
SH
chmod +x "$fixture/launcher/icp"
export NATIVE_ICP_FIXTURE_VERSION="$required_version"

# A launcher on PATH resolves to the native installation for this invocation only.
(
    unset CANIC_TEST_ICP_BIN ICP_CLI_INSTALL_DIR
    export CARGO_HOME="$fixture/cargo" PATH="$fixture/launcher:$original_path"
    use_native_test_icp "$fixture/scratch" "$required_version"
    selected="$(command -v icp)"
    [[ "$selected" == "$fixture/scratch/"*/icp && -L "$selected" ]]
    [[ "$(readlink "$selected")" == "$fixture/cargo/bin/icp" ]]
    [[ "$(icp --version)" == "icp $required_version" ]]
    [[ "$(command -v rustc)" == "$(PATH="$original_path" command -v rustc)" ]]
)
[[ "$PATH" == "$original_path" ]]

# An already selected native tool and an explicit tool path retain authority.
(
    unset CANIC_TEST_ICP_BIN ICP_CLI_INSTALL_DIR
    export CARGO_HOME="$fixture/missing" PATH="$fixture/alternate path:$original_path"
    use_native_test_icp "$fixture/scratch" "$required_version"
    [[ "$(readlink "$(command -v icp)")" == "$fixture/alternate path/icp" ]]
)
(
    export CANIC_TEST_ICP_BIN="$fixture/alternate path/icp"
    use_native_test_icp "$fixture/scratch" "$required_version"
    [[ "$(readlink "$(command -v icp)")" == "$CANIC_TEST_ICP_BIN" ]]
)

# Refusals neither replace PATH nor expose an invocation tool directory.
expect_refusal() {
    local tool="$1" version="$2" scratch="$3"
    local before_path="$PATH" before after
    before="$(find "$fixture/scratch" -mindepth 1 -maxdepth 1 -type d | sort)"
    if CANIC_TEST_ICP_BIN="$tool" NATIVE_ICP_FIXTURE_VERSION="$version" \
        use_native_test_icp "$scratch" "$required_version" > "$fixture/refusal.log" 2>&1; then
        echo "native ICP selection unexpectedly accepted $tool" >&2
        exit 1
    fi
    after="$(find "$fixture/scratch" -mindepth 1 -maxdepth 1 -type d | sort)"
    [[ "$PATH" == "$before_path" && "$before" == "$after" ]]
}
expect_refusal "$fixture/missing" "$required_version" "$fixture/scratch"
expect_refusal "$fixture/launcher/icp" "$required_version" "$fixture/scratch"
expect_refusal "$fixture/cargo/bin/icp" '0.0.0' "$fixture/scratch"
expect_refusal "$fixture/cargo/bin/icp" "$required_version" "$fixture/missing"
ln -s "$fixture/scratch" "$fixture/scratch-link"
expect_refusal "$fixture/cargo/bin/icp" "$required_version" "$fixture/scratch-link"

# Plan-only selection remains effect-free even when the configured binary is absent.
CANIC_TEST_ICP_BIN="$fixture/missing" CANIC_TEST_PLAN_ONLY=1 \
    bash "$ROOT/scripts/ci/run-workspace-tests.sh" targeted-pocketic \
    pic::fleet_registry::baseline::tests::generated_reinstall_recovers_lost_install_and_reaches_working_fleet \
    > "$fixture/plan.log"
[[ "$PATH" == "$original_path" ]]
echo 'native ICP selection tests passed'
