#!/usr/bin/env bash
set -euo pipefail

# Exercise archive integrity and the macOS loader layout without executing
# foreign machine code or contacting a release server.
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-binaryen-install.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/repo/scripts/ci" "$fixture/bin" "$fixture/package/binaryen-version_132/bin" \
    "$fixture/package/binaryen-version_132/lib"
cp "$ROOT/scripts/ci/install-binaryen.sh" "$ROOT/scripts/ci/verify-file-checksum.sh" "$fixture/repo/scripts/ci/"
cat >"$fixture/bin/uname" <<'SH'
#!/usr/bin/env bash
case "$1" in -s) echo "$TEST_OS" ;; -m) echo "$TEST_ARCH" ;; *) exit 2 ;; esac
SH
cat >"$fixture/bin/curl" <<'SH'
#!/usr/bin/env bash
while [[ $# -gt 0 ]]; do
    if [[ "$1" == -o ]]; then cp "$TEST_ARCHIVE" "$2"; exit 0; fi
    shift
done
exit 2
SH
cat >"$fixture/package/binaryen-version_132/bin/wasm-opt" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${TEST_ABORT:-0}" == 1 ]]; then echo 'fixture loader refused library' >&2; exit 134; fi
executable="$0"
[[ ! -L "$executable" ]] || executable="$(readlink "$executable")"
if [[ "$TEST_OS" == Darwin ]]; then
    [[ "$(cat "$(dirname "$executable")/../lib/libbinaryen.dylib")" == library ]] || exit 134
fi
echo 'wasm-opt version 132 (version_132)'
SH
printf 'library\n' >"$fixture/package/binaryen-version_132/lib/libbinaryen.dylib"
chmod +x "$fixture/bin/"* "$fixture/package/binaryen-version_132/bin/wasm-opt"
tar -czf "$fixture/archive.tar.gz" -C "$fixture/package" binaryen-version_132
archive_hash="$(shasum -a 256 "$fixture/archive.tar.gz" | awk '{print $1}')"
binary_hash="$(shasum -a 256 "$fixture/package/binaryen-version_132/bin/wasm-opt" | awk '{print $1}')"
{
    echo 'export CANIC_BINARYEN_VERSION=132'
    for platform in DARWIN_ARM64 DARWIN_X64 LINUX_X64; do
        printf 'export CANIC_BINARYEN_SHA256_%s=%s\n' "$platform" "$archive_hash"
        printf 'export CANIC_BINARYEN_WASM_OPT_SHA256_%s=%s\n' "$platform" "$binary_hash"
    done
} >"$fixture/repo/tool-versions.env"
export PATH="$fixture/bin:$PATH" TEST_ARCHIVE="$fixture/archive.tar.gz"
for host in Darwin:arm64 Darwin:x86_64 Linux:x86_64; do
    export TEST_OS="${host%:*}" TEST_ARCH="${host#*:}"
    export BINARYEN_INSTALL_DIR="$fixture/install-$TEST_OS-$TEST_ARCH"
    installed="$(bash "$fixture/repo/scripts/ci/install-binaryen.sh")"
    [[ "$installed" == "$BINARYEN_INSTALL_DIR/wasm-opt" ]]
    [[ "$("$installed" --version)" == 'wasm-opt version 132 (version_132)' ]]
    # Repeated install preserves the same usable bundle after download scratch is gone.
    bash "$fixture/repo/scripts/ci/install-binaryen.sh" >/dev/null
    "$installed" --version >/dev/null
done
status=0
TEST_ABORT=1 bash "$fixture/repo/scripts/ci/install-binaryen.sh" >"$fixture/abort.log" 2>&1 || status=$?
[[ "$status" == 134 ]]
grep -q 'fixture loader refused library' "$fixture/abort.log"
printf 'corruption' >>"$TEST_ARCHIVE"
if bash "$fixture/repo/scripts/ci/install-binaryen.sh" >"$fixture/checksum.log" 2>&1; then
    echo 'corrupt Binaryen archive accepted' >&2
    exit 1
fi
"$BINARYEN_INSTALL_DIR/wasm-opt" --version >/dev/null
echo 'Binaryen installer: loader layout, repeat install, diagnostics and checksum refusal passed'
