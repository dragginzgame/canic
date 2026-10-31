#!/usr/bin/env bash
set -euo pipefail

# Exercise separate canonical invocations with inert Cargo/gates and tool bytes.
ROOT="$(cd "$(dirname "$0")/../.." && pwd -P)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-build-environment.XXXXXX")"
trap 'if [[ $? == 0 ]]; then rm -rf -- "$fixture"; else echo "Environment fixture retained: $fixture" >&2; fi' EXIT
unset MAKEFLAGS MFLAGS MAKEOVERRIDES GNUMAKEFLAGS MAKEFILES
# This fixture owns its synthetic source; the enclosing release owns a different HEAD.
unset RELEASE_SOURCE
unset CARGO_TARGET_DIR CANIC_CARGO_TARGET_DIR RUSTC_WRAPPER CANIC_SCCACHE_BIN SCCACHE_BIN
unset SCCACHE_RUNTIME_DIR CARGO_INCREMENTAL
ENV_REAL_MAKE="$(command -v make)"
export ENV_REAL_MAKE ENV_EVIDENCE="$fixture/evidence"
mkdir -p "$fixture/"{scripts/ci,ci,make,bin}
cp "$ROOT/Makefile" "$ROOT/tool-versions.env" "$fixture/"
cp "$ROOT/make/"{consumer,tools,execution,release,rust-format}.mk "$fixture/make/"
cp "$ROOT/ci/"{tool-versions.env,ic-tools.tsv} "$fixture/ci/"
cp "$ROOT/scripts/ci/"{prepare-validation,run-release-validation-lane,run-canic-sccache,run-sccache,check-make-execution,ic-tool-pins}.sh "$fixture/scripts/ci/"
cat > "$fixture/scripts/ci/install-pinned-ci-tool.sh" <<'INSTALL'
#!/usr/bin/env bash
set -euo pipefail
source ci/tool-versions.env
[[ "$1" == sccache ]]
mkdir -p "$CANIC_SCCACHE_INSTALL_DIR"
printf '#!/usr/bin/env bash\necho "sccache %s"\n' "$SHARED_TOOLING_SCCACHE_VERSION" > "$CANIC_SCCACHE_INSTALL_DIR/sccache"
chmod +x "$CANIC_SCCACHE_INSTALL_DIR/sccache"
INSTALL
cat > "$fixture/bin/make" <<'MAKE'
#!/usr/bin/env bash
set -euo pipefail
case "$*" in
    '--no-print-directory prepare-build-environment'|'--no-print-directory check-build-environment') exec "$ENV_REAL_MAKE" "$@" ;;
    '--no-print-directory prepare-locked-dependencies'|'--no-print-directory validate')
        printf '%s\n' "$CARGO_TARGET_DIR" "$CANIC_SCCACHE_BIN" "$RUSTC_WRAPPER" "$SCCACHE_RUNTIME_DIR" "$CARGO_INCREMENTAL" > "$ENV_EVIDENCE-$2"
        if [[ "$2" == validate ]]; then [[ "$CARGO_NET_OFFLINE" == true ]]; fi ;;
    '--no-print-directory install-host-tools') touch .host-prepared ;;
    '--no-print-directory ensure-clean'|'--no-print-directory tools-check'|'--no-print-directory install-rust-tools') ;;
    *) exit 99 ;;
esac
MAKE
cat > "$fixture/bin/git" <<'GIT'
#!/usr/bin/env bash
[[ "$*" == 'rev-parse HEAD' ]] || exit 99
printf '%040d\n' 1
GIT
cat > "$fixture/bin/uname" <<'UNAME'
#!/usr/bin/env bash
case "$1" in -s) echo "${ENV_TEST_OS:-Linux}" ;; -m) echo x86_64 ;; *) exit 99 ;; esac
UNAME
chmod +x "$fixture/bin/"*
export PATH="$fixture/bin:$PATH"
cd "$fixture"
"$ENV_REAL_MAKE" --no-print-directory prepare-validation SCOPE=checks
"$ENV_REAL_MAKE" --no-print-directory release-verify
cmp "$ENV_EVIDENCE-prepare-locked-dependencies" "$ENV_EVIDENCE-validate"
printf '%s\n' "$fixture/target/cargo" "$fixture/.tools/canic-sccache/bin/sccache" \
    "$fixture/scripts/ci/run-canic-sccache.sh" "$fixture/.tmp/sccache-$(sed -n 's/export SHARED_TOOLING_SCCACHE_VERSION=//p' ci/tool-versions.env)-runtime" 0 > expected
cmp expected "$ENV_EVIDENCE-validate"
# Explicit supported values survive two independent processes.
mkdir -p 'custom tools'
cp .tools/canic-sccache/bin/sccache 'custom tools/sccache'
export CANIC_SCCACHE_BIN="$fixture/custom tools/sccache" CARGO_TARGET_DIR="$fixture/output with spaces"
export SCCACHE_RUNTIME_DIR="$fixture/runtime override" CARGO_INCREMENTAL=1
"$ENV_REAL_MAKE" --no-print-directory prepare-validation SCOPE=checks
"$ENV_REAL_MAKE" --no-print-directory release-verify
cmp "$ENV_EVIDENCE-prepare-locked-dependencies" "$ENV_EVIDENCE-validate"
printf '%s\n' "$CARGO_TARGET_DIR" "$CANIC_SCCACHE_BIN" "$fixture/scripts/ci/run-canic-sccache.sh" "$SCCACHE_RUNTIME_DIR" 1 > expected
cmp expected "$ENV_EVIDENCE-validate"
# An ambient/explicit wrong version fails before validation, never overwritten.
printf '#!/usr/bin/env bash\necho "sccache 0.18.0"\n' > "$CANIC_SCCACHE_BIN"
rm "$ENV_EVIDENCE-validate"
if "$ENV_REAL_MAKE" --no-print-directory release-verify > refusal 2>&1; then exit 1; fi
[[ ! -e "$ENV_EVIDENCE-validate" ]]
grep -F 'policy requires' refusal >/dev/null
cp .tools/canic-sccache/bin/sccache "$CANIC_SCCACHE_BIN"
RUSTC_WRAPPER="$CANIC_SCCACHE_BIN" "$ENV_REAL_MAKE" --no-print-directory check-build-environment
RUSTC_WRAPPER='' "$ENV_REAL_MAKE" --no-print-directory check-build-environment
if CARGO_INCREMENTAL=invalid "$ENV_REAL_MAKE" --no-print-directory check-build-environment > refusal 2>&1; then exit 1; fi
ln -s "$fixture/runtime override" "$fixture/runtime-link"
if SCCACHE_RUNTIME_DIR="$fixture/runtime-link" "$ENV_REAL_MAKE" --no-print-directory check-build-environment > refusal 2>&1; then exit 1; fi
grep -F 'may not be a symlink' refusal >/dev/null
# Cold non-Linux selection uses the existing Rust owner, after host prerequisites.
# Its child setup exports cannot leak into the subsequent qualification process.
unset CANIC_SCCACHE_BIN CARGO_TARGET_DIR SCCACHE_RUNTIME_DIR CARGO_INCREMENTAL
export ENV_TEST_OS=Darwin
mkdir -p scripts/dev
rm -f .host-prepared
cat > scripts/dev/install-rust-tools.sh <<'RUST'
#!/usr/bin/env bash
set -euo pipefail
source ci/tool-versions.env
[[ -f .host-prepared ]]
binary="$PWD/.tools/rust/fixture/bin/sccache"
if [[ "$*" != *--check* ]]; then
    [[ "${RUSTC_WRAPPER:-}" == '' ]]
    mkdir -p "${binary%/*}"
    printf '#!/usr/bin/env bash\necho "sccache %s"\n' "$SHARED_TOOLING_SCCACHE_VERSION" > "$binary"
    chmod +x "$binary"
fi
[[ -x "$binary" ]]
printf '%s\n' "$binary"
RUST
"$ENV_REAL_MAKE" --no-print-directory prepare-validation SCOPE=checks
"$ENV_REAL_MAKE" --no-print-directory release-verify
cmp "$ENV_EVIDENCE-prepare-locked-dependencies" "$ENV_EVIDENCE-validate"
grep -Fx "$fixture/.tools/rust/fixture/bin/sccache" "$ENV_EVIDENCE-validate" >/dev/null
echo 'Separate preparation/qualification defaults, overrides, pin and runtime admission passed (substitute tools/gates)'

# Real Cargo cleanup must remove the aggregate, including old compiler outputs
# and evidence, while leaving source and invocation-owned scratch alone.
mkdir -p .cargo src .tmp/clean-sentinel
cp "$ROOT/.cargo/config.toml" .cargo/
cp "$ROOT/rust-toolchain.toml" .
cp "$ROOT/scripts/ci/cleanup-release-artifacts.sh" scripts/ci/
printf '[package]\nname = "clean-fixture"\nversion = "0.1.0"\nedition = "2024"\n' > Cargo.toml
touch src/lib.rs .tmp/clean-sentinel/retained
unset CARGO_TARGET_DIR CANIC_CARGO_TARGET_DIR
export RUSTC_WRAPPER=''
for command in cargo clean release-clean; do
    mkdir -p target/{cargo/debug,debug,canic-packages,evidence}
    touch target/cargo/debug/artifact target/debug/old-artifact target/canic-packages/source target/evidence/log
    if [[ "$command" == cargo ]]; then
        cargo clean
    else
        # A build output override must not narrow explicit repository cleanup.
        "$ENV_REAL_MAKE" --no-print-directory "$command" CARGO_TARGET_DIR="$fixture/target/cargo"
    fi
    [[ ! -e target && -f src/lib.rs && -f .tmp/clean-sentinel/retained ]]
done
echo 'Plain Cargo and Make cleanup remove the complete target tree (real Cargo, disposable fixture)'
