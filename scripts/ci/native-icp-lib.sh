#!/usr/bin/env bash
# Select one installed native ICP CLI for the owned PocketIC lane.
# This library does not install tools or change the caller's global environment.

is_native_test_icp() {
    local candidate="$1" magic
    [[ -f "$candidate" && -x "$candidate" ]] || return 1
    magic="$(LC_ALL=C od -An -tx1 -N4 "$candidate" | tr -d ' \n')" || return 1
    case "$magic" in
        7f454c46 | cffaedfe | feedfacf | cafebabe | bebafeca | cafebabf | bfbafeca) return 0;;
        *) return 1;;
    esac
}

use_native_test_icp() {
    local scratch="$1"
    local required_version="$2"
    local candidate version bin_dir
    if [[ -v CANIC_TEST_ICP_BIN ]]; then
        candidate="$CANIC_TEST_ICP_BIN"
    else
        candidate="$(command -v icp || true)"
        if ! is_native_test_icp "$candidate"; then
            candidate="${ICP_CLI_INSTALL_DIR:-${CARGO_HOME:-$HOME/.cargo}/bin}/icp"
        fi
    fi
    if [[ ! -f "$candidate" || ! -x "$candidate" ]]; then
        echo "native ICP CLI is unavailable: $candidate; run make install-dev or set CANIC_TEST_ICP_BIN" >&2
        return 1
    fi
    candidate="$(cd -- "$(dirname -- "$candidate")" && pwd)/$(basename -- "$candidate")" || return 1
    if ! is_native_test_icp "$candidate"; then
        echo "PocketIC requires a native ICP CLI executable, not a launcher: $candidate" >&2
        return 1
    fi
    version="$("$candidate" --version 2>&1)" || {
        echo "native ICP CLI version check failed: $candidate" >&2
        return 1
    }
    case "$version" in
        *" $required_version" | *" $required_version "*) ;;
        *)
            echo "native ICP CLI must match pinned $required_version; found: $version" >&2
            return 1
            ;;
    esac
    [[ -d "$scratch" && ! -L "$scratch" ]] || {
        echo "native ICP CLI selection requires the invocation's private scratch" >&2
        return 1
    }
    bin_dir="$(mktemp -d "$scratch/native-icp.XXXXXX")" || return 1
    ln -s -- "$candidate" "$bin_dir/icp" || return 1
    export PATH="$bin_dir:$PATH"
    hash -r
    echo "==> native ICP CLI: $candidate ($version)"
}
