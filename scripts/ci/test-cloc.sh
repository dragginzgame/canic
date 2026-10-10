#!/usr/bin/env bash
# Shared companions: scripts/dev/cloc.sh
set -euo pipefail

ROOT="$0"
[[ "$ROOT" == /* ]] || ROOT="$PWD/$ROOT"
ROOT="$(cd -P "${ROOT%/*}/../.." && printf '%s/.' "$PWD")"
ROOT="${ROOT%/.}"
# The independent fixture owns its default output selection. Individual cases
# below still select explicit targets; an enclosing consumer must not select it.
unset CARGO_TARGET_DIR
FIXTURE="$(mktemp -d "${TMPDIR:-/tmp}/shared-tooling-cloc-test.XXXXXX")"
# Bash 3.2 can enter EXIT with status zero after nounset; require completion too.
fixture_complete=false
finish() {
    local status=$?
    [[ "$fixture_complete" == true || "$status" != 0 ]] || status=1
    if [[ "$status" == 0 ]]; then rm -rf "$FIXTURE"
    else printf "Failed cloc fixture retained: %s\n" "$FIXTURE" >&2; fi
    exit "$status"
}
trap finish EXIT

for command in cargo cloc jq; do
    command -v "$command" >/dev/null 2>&1 || {
        echo "cloc test requires $command" >&2
        exit 1
    }
done

mkdir -p \
    "$FIXTURE/.cargo" \
    "$FIXTURE/crates/alpha/src" \
    "$FIXTURE/crates/alpha/tests" \
    "$FIXTURE/crates/beta/src"
# Cargo also reads ancestor configuration, independently of Git discovery.
printf '[build]\ntarget-dir = "target"\n' > "$FIXTURE/.cargo/config.toml"

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

output="$(bash "$ROOT/scripts/dev/cloc.sh" --manifest "$FIXTURE/Cargo.toml" "$FIXTURE")"

alpha_row="$(printf '%s\n' "$output" | awk '$1 == "alpha" { print $0 }')"
beta_row="$(printf '%s\n' "$output" | awk '$1 == "beta" { print $0 }')"
total_row="$(printf '%s\n' "$output" | awk '$1 == "TOTAL" { print $0 }')"

read -r _ _ alpha_test_loc _ alpha_test_fns alpha_inline_fns <<<"$alpha_row"
read -r _ _ _ _ beta_test_fns beta_inline_fns <<<"$beta_row"
read -r _ _ _ _ total_test_fns total_inline_fns <<<"$total_row"

[[ "$alpha_test_loc" -gt 0 ]] || exit 1
[[ "$alpha_test_fns" -eq 2 && "$alpha_inline_fns" -eq 1 ]] || exit 1
[[ "$beta_test_fns" -eq 1 && "$beta_inline_fns" -eq 1 ]] || exit 1
[[ "$total_test_fns" -eq 3 && "$total_inline_fns" -eq 2 ]] || exit 1

# Build outputs never become maintained source, including a configured target
# inside a member. Literal metacharacters must not prune a similarly named path.
generated_rust() {
    mkdir -p "$1/debug/build/example/out" "$1/tests"
    printf 'pub fn generated() {}\n#[test]\nfn generated_test() {}\n' > "$1/debug/build/example/out/generated.rs"
    printf '#[test]\nfn generated_integration_test() {}\n' > "$1/tests/generated.rs"
}
generated_rust "$FIXTURE/target"
[[ "$(bash "$ROOT/scripts/dev/cloc.sh" --manifest "$FIXTURE/Cargo.toml" "$FIXTURE")" == "$output" ]] || exit 1
custom_target="$FIXTURE/crates/alpha/build [generated]*?"
generated_rust "$custom_target"
[[ "$(CARGO_TARGET_DIR="$custom_target" bash "$ROOT/scripts/dev/cloc.sh" --manifest "$FIXTURE/Cargo.toml" "$FIXTURE")" == "$output" ]] || exit 1
# This source directory matches the unescaped target glob and must be counted.
mkdir -p "$FIXTURE/crates/alpha/build generated-copy"
printf 'pub fn maintained() {}\n' > "$FIXTURE/crates/alpha/build generated-copy/lib.rs"
custom_output="$(CARGO_TARGET_DIR="$custom_target" bash "$ROOT/scripts/dev/cloc.sh" --manifest "$FIXTURE/Cargo.toml" "$FIXTURE")"
read -r _ before_runtime _ _ _ _ <<<"$alpha_row"
read -r _ after_runtime _ _ after_tests _ <<<"$(printf '%s\n' "$custom_output" | awk '$1 == "alpha"')"
[[ "$after_runtime" == "$((before_runtime + 1))" && "$after_tests" == 2 ]] || exit 1
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

nested_output="$(bash "$ROOT/scripts/dev/cloc.sh" --manifest "$FIXTURE/Cargo.toml" "$FIXTURE")"
root_row="$(printf '%s\n' "$nested_output" | awk '$1 == "root_package" { print $0 }')"
total_row="$(printf '%s\n' "$nested_output" | awk '$1 == "TOTAL" { print $0 }')"
read -r _ root_runtime_loc root_test_loc _ root_test_fns root_inline_fns <<<"$root_row"
read -r _ total_runtime_loc total_test_loc _ total_test_fns total_inline_fns <<<"$total_row"
[[ "$root_runtime_loc" -eq 3 && "$root_test_loc" -eq 2 ]] || exit 1
[[ "$root_test_fns" -eq 2 && "$root_inline_fns" -eq 1 ]] || exit 1
[[ "$total_runtime_loc" -eq $((member_runtime_loc + 3)) ]] || exit 1
[[ "$total_test_loc" -eq $((member_test_loc + 2)) ]] || exit 1
[[ "$total_test_fns" -eq 5 && "$total_inline_fns" -eq 3 ]] || exit 1
generated_rust "$FIXTURE/target"
[[ "$(bash "$ROOT/scripts/dev/cloc.sh" --manifest "$FIXTURE/Cargo.toml" "$FIXTURE")" == "$nested_output" ]] || exit 1
custom_target="$FIXTURE/build [generated]*?"
# Move only fixture-owned output out of the default target before changing its
# configured identity; Cargo reports one selected target directory at a time.
mv "$FIXTURE/target" "$custom_target"
[[ "$(CARGO_TARGET_DIR="$custom_target" bash "$ROOT/scripts/dev/cloc.sh" --manifest "$FIXTURE/Cargo.toml" "$FIXTURE")" == "$nested_output" ]] || exit 1
# Existing output selected through a directory alias still has one physical
# identity. Both direct and ancestor symlinks must exclude those same bytes.
ln -s "$custom_target" "$FIXTURE/output-link"
[[ "$(CARGO_TARGET_DIR="$FIXTURE/output-link" bash "$ROOT/scripts/dev/cloc.sh" --manifest "$FIXTURE/Cargo.toml" "$FIXTURE")" == "$nested_output" ]] || exit 1
rm "$FIXTURE/output-link"
ln -s "$FIXTURE" "$FIXTURE/parent-link"
[[ "$(CARGO_TARGET_DIR="$FIXTURE/parent-link/${custom_target##*/}" bash "$ROOT/scripts/dev/cloc.sh" --manifest "$FIXTURE/Cargo.toml" "$FIXTURE")" == "$nested_output" ]] || exit 1
rm "$FIXTURE/parent-link"
rm -r "$custom_target"

# Checkout ancestors must not affect runtime/test classification. Spaces and
# glob characters in paths must remain literal when excluding nested members.
relocated="$FIXTURE/tests/checkout [copy]"
mkdir -p "$relocated"
cp "$FIXTURE/Cargo.toml" "$relocated/"
cp -R "$FIXTURE/src" "$FIXTURE/crates" "$FIXTURE/.cargo" "$relocated/"
mkdir -p "$relocated/tests"
cp "$FIXTURE/tests/root.rs" "$relocated/tests/"
relocated_output="$(bash "$ROOT/scripts/dev/cloc.sh" --manifest "$relocated/Cargo.toml" "$relocated")"
[[ "$relocated_output" == "$nested_output" ]] || exit 1

# Explicit independent-workspace selection keeps graphs and build output separate.
independent="$FIXTURE/independent"
mkdir -p "$independent/.cargo" "$independent/crates/probe/src"
cat > "$independent/Cargo.toml" <<'TOML'
[workspace]
members = ["crates/probe"]
resolver = "2"
TOML
cat > "$independent/crates/probe/Cargo.toml" <<'TOML'
[package]
name = "independent-probe"
version = "0.1.0"
edition = "2021"
TOML
printf 'pub fn probe() {}\n' > "$independent/crates/probe/src/lib.rs"
printf '[build]\ntarget-dir = "crates/probe/output"\n' > "$independent/.cargo/config.toml"
git init -q "$FIXTURE"
(cd "$independent" && cargo generate-lockfile --offline)
selected_before="$(bash "$ROOT/scripts/dev/cloc.sh" --manifest "$independent/Cargo.toml" "$FIXTURE")"
generated_rust "$independent/crates/probe/output"
selected_after="$(bash "$ROOT/scripts/dev/cloc.sh" --manifest "$independent/Cargo.toml" "$FIXTURE")"
[[ "$selected_before" == "$selected_after" ]] || exit 1
[[ "$(printf '%s\n' "$selected_after" | awk '$1 == "independent-probe" { print $2,$3,$5,$6 }')" == '1 0 0 0' ]] || exit 1
[[ "$(printf '%s\n' "$selected_after" | awk '$1 == "alpha" { print }')" == '' ]] || exit 1
echo 'cloc tests, including independent workspace selection, passed'
fixture_complete=true
