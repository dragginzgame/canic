#!/usr/bin/env bash
set -euo pipefail

# Canic selects pins and destinations; Shared Tooling owns installation mechanics.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
# shellcheck source=/dev/null
source "$ROOT/tool-versions.env"
[[ $# == 1 ]] || { echo 'usage: install-pinned-ci-tool.sh <actionlint|shellcheck|sccache>' >&2; exit 2; }
tool="$1"
case "$tool" in
    actionlint) prefix=CANIC_ACTIONLINT; directory="${ACTIONLINT_INSTALL_DIR:-$HOME/.local/bin}" ;;
    shellcheck) prefix=CANIC_SHELLCHECK; directory="${SHELLCHECK_INSTALL_DIR:-$HOME/.local/bin}" ;;
    sccache)
        [[ "$(uname -s):$(uname -m)" == Linux:x86_64 ]] || {
            echo 'CI sccache installer requires Linux x86_64' >&2; exit 1;
        }
        prefix=CANIC_SCCACHE; directory="${CANIC_SCCACHE_INSTALL_DIR:-$HOME/.local/bin}" ;;
    *) echo 'unknown Canic CI tool' >&2; exit 2 ;;
esac
case "$(uname -s):$(uname -m)" in
    Linux:x86_64|Linux:amd64) suffix=LINUX_AMD64 ;;
    Linux:arm64|Linux:aarch64) suffix=LINUX_ARM64 ;;
    Darwin:x86_64|Darwin:amd64) suffix=DARWIN_AMD64 ;;
    Darwin:arm64|Darwin:aarch64) suffix=DARWIN_ARM64 ;;
    *) echo "unsupported $tool host" >&2; exit 1 ;;
esac
if [[ "$tool" == shellcheck ]]; then
    case "$suffix" in
        LINUX_AMD64) suffix=LINUX_X86_64 ;; LINUX_ARM64) suffix=LINUX_AARCH64 ;;
        DARWIN_AMD64) suffix=DARWIN_X86_64 ;; DARWIN_ARM64) suffix=DARWIN_AARCH64 ;;
    esac
elif [[ "$tool" == sccache ]]; then suffix=LINUX_X64; fi
version_key="${prefix}_VERSION"
digest_key="${prefix}_SHA256_${suffix}"
binary="$(bash "$ROOT/scripts/ci/install-$tool.sh" --version "${!version_key}" \
    --sha256 "${!digest_key}" --install-dir "$directory")"
if [[ "$tool" == sccache && -n "${GITHUB_PATH:-}" ]]; then
    printf '%s\n' "$directory" >> "$GITHUB_PATH"
fi
printf '%s\n' "$binary"
