#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TOOLS="$ROOT_DIR/tool-versions.env"
INSTALLING="$ROOT_DIR/INSTALLING.md"
LATEST_RELEASE_URL="https://github.com/dfinity/icp-cli/releases/latest"
TMP_DIR=""

fail() {
    echo "ICP CLI pin update failed: $1" >&2
    exit 1
}

resolve_latest_version() {
    local resolved_url
    local tag

    resolved_url="$(
        curl --proto '=https' --proto-redir '=https' --tlsv1.2 \
            --fail --silent --show-error --location \
            --retry 3 --retry-all-errors --connect-timeout 15 --max-time 120 \
            --output /dev/null --write-out '%{url_effective}' \
            "$LATEST_RELEASE_URL"
    )"
    case "$resolved_url" in
    https://github.com/dfinity/icp-cli/releases/tag/v*) ;;
    *) fail "unexpected latest-release URL: $resolved_url" ;;
    esac

    tag="${resolved_url##*/}"
    latest_version="${tag#v}"
    [[ "$latest_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] ||
        fail "latest release tag is not a stable semantic version: $tag"
}

checksum_for() {
    local archive="$1"
    local digest

    digest="$(
        awk -v expected="$archive" '
            {
                filename = $2
                sub(/^\*/, "", filename)
                if (filename == expected) {
                    print $1
                }
            }
        ' "$TMP_DIR/sha256.sum"
    )"
    [[ "$digest" =~ ^[0-9a-f]{64}$ ]] ||
        fail "missing or invalid official checksum for $archive"
    printf '%s\n' "$digest"
}

write_updated_tools() {
    local output="$1"

    awk \
        -v version="$latest_version" \
        -v darwin_arm64="$darwin_arm64" \
        -v linux_arm64="$linux_arm64" \
        -v darwin_x86_64="$darwin_x86_64" \
        -v linux_x86_64="$linux_x86_64" '
        index($0, "export CANIC_ICP_CLI_VERSION=") == 1 {
            print "export CANIC_ICP_CLI_VERSION=" version
            seen_version += 1
            next
        }
        index($0, "export CANIC_ICP_CLI_SHA256_AARCH64_APPLE_DARWIN=") == 1 {
            print "export CANIC_ICP_CLI_SHA256_AARCH64_APPLE_DARWIN=" darwin_arm64
            seen_darwin_arm64 += 1
            next
        }
        index($0, "export CANIC_ICP_CLI_SHA256_AARCH64_UNKNOWN_LINUX_GNU=") == 1 {
            print "export CANIC_ICP_CLI_SHA256_AARCH64_UNKNOWN_LINUX_GNU=" linux_arm64
            seen_linux_arm64 += 1
            next
        }
        index($0, "export CANIC_ICP_CLI_SHA256_X86_64_APPLE_DARWIN=") == 1 {
            print "export CANIC_ICP_CLI_SHA256_X86_64_APPLE_DARWIN=" darwin_x86_64
            seen_darwin_x86_64 += 1
            next
        }
        index($0, "export CANIC_ICP_CLI_SHA256_X86_64_UNKNOWN_LINUX_GNU=") == 1 {
            print "export CANIC_ICP_CLI_SHA256_X86_64_UNKNOWN_LINUX_GNU=" linux_x86_64
            seen_linux_x86_64 += 1
            next
        }
        { print }
        END {
            if (seen_version != 1 ||
                seen_darwin_arm64 != 1 ||
                seen_linux_arm64 != 1 ||
                seen_darwin_x86_64 != 1 ||
                seen_linux_x86_64 != 1) {
                exit 42
            }
        }
    ' "$TOOLS" >"$output" || fail "canonical ICP CLI pin fields are incomplete"
}

write_updated_installing() {
    local output="$1"

    awk -v current="$current_version" -v latest="$latest_version" '
        {
            needle = "maintainer toolchain currently pins `" current "`"
            offset = index($0, needle)
            if (offset > 0) {
                replacement = "maintainer toolchain currently pins `" latest "`"
                $0 = substr($0, 1, offset - 1) replacement \
                    substr($0, offset + length(needle))
                replacements += 1
            }
            print
        }
        END {
            if (replacements != 1) {
                exit 42
            }
        }
    ' "$INSTALLING" >"$output" ||
        fail "installation guidance does not contain the current pin"
}

main() {
    local current_major
    local latest_major
    local release_base

    if [ "$#" -ne 0 ]; then
        fail "usage: scripts/dev/update-icp-cli-pin.sh"
    fi

    # shellcheck source=/dev/null
    source "$TOOLS"
    current_version="${CANIC_ICP_CLI_VERSION:-}"
    [[ "$current_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] ||
        fail "tool-versions.env does not contain an exact ICP CLI pin"

    if ! command -v curl >/dev/null 2>&1; then
        fail "curl is required"
    fi
    resolve_latest_version

    current_major="${current_version%%.*}"
    latest_major="${latest_version%%.*}"
    [ "$latest_major" = "$current_major" ] ||
        fail "latest release $latest_version crosses supported major $current_major; review compatibility explicitly"

    if [ "$latest_version" = "$current_version" ]; then
        printf 'ICP CLI pin is current: %s\n' "$current_version"
        return 0
    fi

    TMP_DIR="$(mktemp -d)"
    trap 'rm -rf "$TMP_DIR"' EXIT
    release_base="https://github.com/dfinity/icp-cli/releases/download/v$latest_version"
    curl --proto '=https' --proto-redir '=https' --tlsv1.2 \
        --fail --silent --show-error --location \
        --retry 3 --retry-all-errors --connect-timeout 15 --max-time 120 \
        --output "$TMP_DIR/sha256.sum" "$release_base/sha256.sum"

    darwin_arm64="$(checksum_for icp-cli-aarch64-apple-darwin.tar.xz)"
    linux_arm64="$(checksum_for icp-cli-aarch64-unknown-linux-gnu.tar.xz)"
    darwin_x86_64="$(checksum_for icp-cli-x86_64-apple-darwin.tar.xz)"
    linux_x86_64="$(checksum_for icp-cli-x86_64-unknown-linux-gnu.tar.xz)"

    write_updated_tools "$TMP_DIR/tool-versions.env"
    write_updated_installing "$TMP_DIR/INSTALLING.md"
    chmod 0644 "$TMP_DIR/tool-versions.env" "$TMP_DIR/INSTALLING.md"
    mv "$TMP_DIR/tool-versions.env" "$TOOLS"
    mv "$TMP_DIR/INSTALLING.md" "$INSTALLING"

    printf 'Pinned ICP CLI %s -> %s using the official release checksums.\n' \
        "$current_version" "$latest_version"
}

main "$@"
