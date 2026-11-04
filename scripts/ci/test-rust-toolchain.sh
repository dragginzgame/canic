#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd -P)"
export PATH="$root/.tools/host/bin:$PATH"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-compiler-policy.XXXXXX")"
trap 'if [[ $? == 0 ]]; then rm -rf -- "$fixture"; else echo "Compiler policy fixture retained: $fixture" >&2; fi' EXIT
mkdir -p "$fixture/scripts/ci" "$fixture/bin"
cp "$root/scripts/ci/"{check-rust-toolchain,read-cargo-workspace-rust-version}.sh "$fixture/scripts/ci/"
printf '[workspace.package]\nrust-version = "1.99.1"\n' > "$fixture/Cargo.toml"
printf '[toolchain]\nchannel = "1.99.1"\n' > "$fixture/rust-toolchain.toml"
for consumer in consumer embedded-consumer; do
    mkdir -p "$fixture/integrations/blob-service/$consumer"
    cp "$fixture/Cargo.toml" "$fixture/integrations/blob-service/$consumer/Cargo.toml"
done
cat > "$fixture/bin/rustc" <<'SH'
#!/usr/bin/env bash
[[ "${RUSTUP_AUTO_INSTALL:-}" == 0 ]] || exit 91
printf 'rustc %s (fixture)\nrelease: %s\n' "${COMPILER_FIXTURE_RUST:-1.99.1}" "${COMPILER_FIXTURE_RUST:-1.99.1}"
SH
cat > "$fixture/bin/cargo" <<'SH'
#!/usr/bin/env bash
[[ "${RUSTUP_AUTO_INSTALL:-}" == 0 ]] || exit 91
case "$*" in
    '--version --verbose') printf 'cargo %s (fixture)\n' "${COMPILER_FIXTURE_CARGO:-1.99.1}" ;;
    'metadata --locked --offline --no-deps --format-version 1')
        printf '{"workspace_members":["fixture"],"packages":[{"id":"fixture","rust_version":"%s"}]}\n' "${COMPILER_FIXTURE_MEMBER:-1.99.1}" ;;
    *) exit 92 ;;
esac
SH
printf '#!/usr/bin/env bash\nprintf "rustc 1.91.0 (fixture)\\nrelease: 1.91.0\\n"\n' > "$fixture/bin/old-rustc"
chmod +x "$fixture/bin/"*
export PATH="$fixture/bin:$PATH"
unset RUSTC
bash "$fixture/scripts/ci/check-rust-toolchain.sh" > "$fixture/output"
for command in rustc cargo; do
    if [[ "$command" == rustc ]]; then
        if COMPILER_FIXTURE_RUST=1.91.0 bash "$fixture/scripts/ci/check-rust-toolchain.sh" > "$fixture/output" 2>&1; then exit 1; fi
    else
        if COMPILER_FIXTURE_CARGO=1.91.0 bash "$fixture/scripts/ci/check-rust-toolchain.sh" > "$fixture/output" 2>&1; then exit 1; fi
    fi
done
if RUSTC="$fixture/bin/old-rustc" bash "$fixture/scripts/ci/check-rust-toolchain.sh" > "$fixture/output" 2>&1; then exit 1; fi
if RUSTC='' bash "$fixture/scripts/ci/check-rust-toolchain.sh" > "$fixture/output" 2>&1; then exit 1; fi
if COMPILER_FIXTURE_MEMBER=1.99.0 bash "$fixture/scripts/ci/check-rust-toolchain.sh" > "$fixture/output" 2>&1; then exit 1; fi
printf '[toolchain]\nchannel = "1.91.0"\n' > "$fixture/rust-toolchain.toml"
if bash "$fixture/scripts/ci/check-rust-toolchain.sh" > "$fixture/output" 2>&1; then exit 1; fi
printf '[toolchain]\nchannel = "1.99.1"\n' > "$fixture/rust-toolchain.toml"
printf '[workspace.package]\nrust-version = "1.91.0"\n' > "$fixture/integrations/blob-service/consumer/Cargo.toml"
if bash "$fixture/scripts/ci/check-rust-toolchain.sh" > "$fixture/output" 2>&1; then exit 1; fi
echo 'Compiler policy checks actual rustc/Cargo, explicit RUSTC, pinned channel and managed floors without installation'
