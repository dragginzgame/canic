#!/usr/bin/env bash
set -euo pipefail

# Prove caller pin/platform/path admission without downloading or replacing tools.
ROOT="$(cd "$(dirname "$0")/../.." && pwd -P)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-pinned-tools.XXXXXX")"
trap 'if [[ $? == 0 ]]; then rm -rf -- "$fixture"; else echo "Pinned tool fixture retained: $fixture" >&2; fi' EXIT
mkdir -p "$fixture/scripts/ci" "$fixture/bin"
cp "$ROOT/scripts/ci/install-pinned-ci-tool.sh" "$fixture/scripts/ci/"
cp "$ROOT/tool-versions.env" "$fixture/tool-versions.env"
# shellcheck source=/dev/null
source "$fixture/tool-versions.env"
export PIN_EVENTS="$fixture/events"
cat > "$fixture/bin/uname" <<'SH'
#!/usr/bin/env bash
case "$1" in -s) echo "$PIN_OS" ;; -m) echo "$PIN_ARCH" ;; *) exit 99 ;; esac
SH
for tool in actionlint shellcheck sccache; do
    cat > "$fixture/scripts/ci/install-$tool.sh" <<'SH'
#!/usr/bin/env bash
printf '%s\0' "$@" > "$PIN_EVENTS"
printf '%s\n' "$PIN_DEST/tool"
exit "${PIN_INSTALL_STATUS:-0}"
SH
done
chmod +x "$fixture/bin/uname"
export ACTIONLINT_INSTALL_DIR="$fixture/with spaces/actionlint"
export SHELLCHECK_INSTALL_DIR="$fixture/with spaces/shellcheck"
export CANIC_SCCACHE_INSTALL_DIR="$fixture/with spaces/sccache"
export GITHUB_PATH="$fixture/github-path"
for host in Linux:x86_64 Linux:aarch64 Darwin:x86_64 Darwin:arm64; do
    export PIN_OS="${host%:*}" PIN_ARCH="${host#*:}"
    case "$host" in
        Linux:x86_64) al=LINUX_AMD64; sc=LINUX_X86_64 ;;
        Linux:aarch64) al=LINUX_ARM64; sc=LINUX_AARCH64 ;;
        Darwin:x86_64) al=DARWIN_AMD64; sc=DARWIN_X86_64 ;;
        Darwin:arm64) al=DARWIN_ARM64; sc=DARWIN_AARCH64 ;;
    esac
    for tool in actionlint shellcheck; do
        case "$tool" in
            actionlint) version="$CANIC_ACTIONLINT_VERSION"; key="CANIC_ACTIONLINT_SHA256_$al"; export PIN_DEST="$ACTIONLINT_INSTALL_DIR" ;;
            shellcheck) version="$CANIC_SHELLCHECK_VERSION"; key="CANIC_SHELLCHECK_SHA256_$sc"; export PIN_DEST="$SHELLCHECK_INSTALL_DIR" ;;
        esac
        PATH="$fixture/bin:$PATH" bash "$fixture/scripts/ci/install-pinned-ci-tool.sh" "$tool" > "$fixture/output"
        printf '%s\0' --version "$version" --sha256 "${!key}" --install-dir "$PIN_DEST" > "$fixture/expected"
        cmp "$fixture/expected" "$PIN_EVENTS"
        [[ "$(cat "$fixture/output")" == "$PIN_DEST/tool" ]]
    done
done
export PIN_OS=Linux PIN_ARCH=x86_64 PIN_DEST="$CANIC_SCCACHE_INSTALL_DIR"
PATH="$fixture/bin:$PATH" bash "$fixture/scripts/ci/install-pinned-ci-tool.sh" sccache > "$fixture/output"
printf '%s\0' --version "$CANIC_SCCACHE_VERSION" --sha256 "$CANIC_SCCACHE_SHA256_LINUX_X64" \
    --install-dir "$PIN_DEST" > "$fixture/expected"
cmp "$fixture/expected" "$PIN_EVENTS"
[[ "$(cat "$GITHUB_PATH")" == "$PIN_DEST" ]]
for host in Darwin:arm64 Linux:aarch64; do
    export PIN_OS="${host%:*}" PIN_ARCH="${host#*:}"
    rm -f "$PIN_EVENTS"
    if PATH="$fixture/bin:$PATH" bash "$fixture/scripts/ci/install-pinned-ci-tool.sh" sccache > "$fixture/output" 2>&1; then exit 1; fi
    [[ ! -e "$PIN_EVENTS" ]]
done
export PIN_OS=Linux PIN_ARCH=x86_64
status=0
PATH="$fixture/bin:$PATH" PIN_INSTALL_STATUS=23 bash "$fixture/scripts/ci/install-pinned-ci-tool.sh" sccache \
    > "$fixture/output" 2>&1 || status=$?
[[ "$status" == 23 ]]
[[ "$(cat "$GITHUB_PATH")" == "$PIN_DEST" ]]
echo 'Canic pin/platform/argv and failed-install propagation checks passed (no downloads)'
