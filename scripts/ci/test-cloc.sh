#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd -P)"
FIXTURE="$(mktemp -d "${TMPDIR:-/tmp}/shared-tooling-cloc-test.XXXXXX")"
trap 'if [[ $? == 0 ]]; then rm -rf "$FIXTURE"; else printf "Failed cloc fixture retained: %s\n" "$FIXTURE" >&2; fi' EXIT

for command in cargo cloc jq; do
    command -v "$command" >/dev/null 2>&1 || {
        echo "cloc test requires $command" >&2
        exit 1
    }
done

mkdir -p \
    "$FIXTURE/crates/alpha/src" \
    "$FIXTURE/crates/alpha/tests" \
    "$FIXTURE/crates/beta/src"

cat >"$FIXTURE/Cargo.toml" <<'TOML'
[workspace]
members = ["crates/alpha", "crates/beta"]
resolver = "2"
TOML

cat >"$FIXTURE/crates/alpha/Cargo.toml" <<'TOML'
[package]
name = "alpha"
version = "0.1.0"
edition = "2021"
TOML

cat >"$FIXTURE/crates/alpha/src/lib.rs" <<'RUST'
pub fn value() -> u8 {
    1
}

#[cfg(test)]
mod tests {
    #[test]
    fn inline_test() {}

    #[testing]
    fn similarly_named_non_test_attribute() {}
}
RUST

cat >"$FIXTURE/crates/alpha/tests/public.rs" <<'RUST'
#[test]
fn integration_test() {}
RUST

cat >"$FIXTURE/crates/beta/Cargo.toml" <<'TOML'
[package]
name = "beta"
version = "0.1.0"
edition = "2021"
TOML

cat >"$FIXTURE/crates/beta/src/lib.rs" <<'RUST'
#[tokio::test(flavor = "current_thread")]
async fn async_test() {}
RUST

output="$(bash "$ROOT/scripts/dev/cloc.sh" "$FIXTURE")"

alpha_row="$(printf '%s\n' "$output" | awk '$1 == "alpha" { print $0 }')"
beta_row="$(printf '%s\n' "$output" | awk '$1 == "beta" { print $0 }')"
total_row="$(printf '%s\n' "$output" | awk '$1 == "TOTAL" { print $0 }')"

read -r _ _ alpha_test_loc _ alpha_test_fns alpha_inline_fns <<<"$alpha_row"
read -r _ _ _ _ beta_test_fns beta_inline_fns <<<"$beta_row"
read -r _ _ _ _ total_test_fns total_inline_fns <<<"$total_row"

[[ "$alpha_test_loc" -gt 0 ]]
[[ "$alpha_test_fns" -eq 2 && "$alpha_inline_fns" -eq 1 ]]
[[ "$beta_test_fns" -eq 1 && "$beta_inline_fns" -eq 1 ]]
[[ "$total_test_fns" -eq 3 && "$total_inline_fns" -eq 2 ]]

# Build outputs never become maintained source, including a configured target
# inside a member. Literal metacharacters must not prune a similarly named path.
generated_rust() {
    mkdir -p "$1/debug/build/example/out" "$1/tests"
    printf 'pub fn generated() {}\n#[test]\nfn generated_test() {}\n' > "$1/debug/build/example/out/generated.rs"
    printf '#[test]\nfn generated_integration_test() {}\n' > "$1/tests/generated.rs"
}
generated_rust "$FIXTURE/target"
[[ "$(bash "$ROOT/scripts/dev/cloc.sh" "$FIXTURE")" == "$output" ]]
custom_target="$FIXTURE/crates/alpha/build [generated]*?"
generated_rust "$custom_target"
[[ "$(CARGO_TARGET_DIR="$custom_target" bash "$ROOT/scripts/dev/cloc.sh" "$FIXTURE")" == "$output" ]]
# This source directory matches the unescaped target glob and must be counted.
mkdir -p "$FIXTURE/crates/alpha/build generated-copy"
printf 'pub fn maintained() {}\n' > "$FIXTURE/crates/alpha/build generated-copy/lib.rs"
custom_output="$(CARGO_TARGET_DIR="$custom_target" bash "$ROOT/scripts/dev/cloc.sh" "$FIXTURE")"
read -r _ before_runtime _ _ _ _ <<<"$alpha_row"
read -r _ after_runtime _ _ after_tests _ <<<"$(printf '%s\n' "$custom_output" | awk '$1 == "alpha"')"
[[ "$after_runtime" == "$((before_runtime + 1))" && "$after_tests" == 2 ]]
rm -r "$custom_target" "$FIXTURE/crates/alpha/build generated-copy"

# A workspace root can also be a package, with other members below it.
read -r _ member_runtime_loc member_test_loc _ _ _ <<<"$total_row"
cat >>"$FIXTURE/Cargo.toml" <<'TOML'

[package]
name = "root_package"
version = "0.1.0"
edition = "2021"
TOML
mkdir -p "$FIXTURE/src" "$FIXTURE/tests"
cat >"$FIXTURE/src/lib.rs" <<'RUST'
pub fn root_value() -> u8 { 3 }
#[test]
fn root_inline_test() {}
RUST
cat >"$FIXTURE/tests/root.rs" <<'RUST'
#[test]
fn root_integration_test() {}
RUST

nested_output="$(bash "$ROOT/scripts/dev/cloc.sh" "$FIXTURE")"
root_row="$(printf '%s\n' "$nested_output" | awk '$1 == "root_package" { print $0 }')"
total_row="$(printf '%s\n' "$nested_output" | awk '$1 == "TOTAL" { print $0 }')"
read -r _ root_runtime_loc root_test_loc _ root_test_fns root_inline_fns <<<"$root_row"
read -r _ total_runtime_loc total_test_loc _ total_test_fns total_inline_fns <<<"$total_row"
[[ "$root_runtime_loc" -eq 3 && "$root_test_loc" -eq 2 ]]
[[ "$root_test_fns" -eq 2 && "$root_inline_fns" -eq 1 ]]
[[ "$total_runtime_loc" -eq $((member_runtime_loc + 3)) ]]
[[ "$total_test_loc" -eq $((member_test_loc + 2)) ]]
[[ "$total_test_fns" -eq 5 && "$total_inline_fns" -eq 3 ]]
generated_rust "$FIXTURE/target"
[[ "$(bash "$ROOT/scripts/dev/cloc.sh" "$FIXTURE")" == "$nested_output" ]]
custom_target="$FIXTURE/build [generated]*?"
# Move only fixture-owned output out of the default target before changing its
# configured identity; Cargo reports one selected target directory at a time.
mv "$FIXTURE/target" "$custom_target"
[[ "$(CARGO_TARGET_DIR="$custom_target" bash "$ROOT/scripts/dev/cloc.sh" "$FIXTURE")" == "$nested_output" ]]
rm -r "$custom_target"

# Checkout ancestors must not affect runtime/test classification. Spaces and
# glob characters in paths must remain literal when excluding nested members.
relocated="$FIXTURE/tests/checkout [copy]"
mkdir -p "$relocated"
cp "$FIXTURE/Cargo.toml" "$relocated/"
cp -R "$FIXTURE/src" "$FIXTURE/crates" "$relocated/"
mkdir -p "$relocated/tests"
cp "$FIXTURE/tests/root.rs" "$relocated/tests/"
relocated_output="$(bash "$ROOT/scripts/dev/cloc.sh" "$relocated")"
[[ "$relocated_output" == "$nested_output" ]]

echo "cloc tests passed"
