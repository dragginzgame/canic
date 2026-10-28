#!/usr/bin/env bash
set -euo pipefail

# Canic selects preparation scopes; existing pinned installers own tool bytes.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
cd "$ROOT"
# shellcheck source=tool-versions.env
source "$ROOT/tool-versions.env"
# shellcheck source=ci/tool-versions.env
source "$ROOT/ci/tool-versions.env"
case "${1:-}" in
    release|preflight|security|checks|environment|check-environment) [[ $# == 1 ]] || exit 2 ;;
    *) echo 'usage: prepare-validation.sh <release|preflight|security|checks|environment|check-environment>' >&2; exit 2 ;;
esac

# Make owns defaults on every invocation; setup subprocess exports are not state.
if [[ "$1" == environment || "$1" == check-environment ]]; then
    [[ -n "${CARGO_TARGET_DIR:-}" ]] || { echo 'CARGO_TARGET_DIR must select a compiler output directory' >&2; exit 1; }
    case "${CARGO_INCREMENTAL:-0}" in
        0|1) ;;
        *) echo 'CARGO_INCREMENTAL must be 0 or 1' >&2; exit 1 ;;
    esac
    case "${RUSTC_WRAPPER:-}" in
        '') exit 0 ;; # Explicitly disabling a wrapper remains supported.
        "$ROOT/scripts/ci/run-canic-sccache.sh")
            selected="${CANIC_SCCACHE_BIN:-}"
            if [[ "$1" == environment ]]; then
                if [[ "$(uname -s):$(uname -m)" == Linux:x86_64 ]]; then
                    managed="$ROOT/.tools/canic-sccache/bin/sccache"
                    if [[ "$selected" == "$managed" ]]; then
                        CANIC_SCCACHE_INSTALL_DIR="${managed%/*}" \
                            bash scripts/ci/install-pinned-ci-tool.sh sccache >/dev/null
                    fi
                elif [[ -z "$selected" ]]; then
                    # Existing installer owns the exact selection and its path.
                    make --no-print-directory install-host-tools
                    selected="$(RUSTC_WRAPPER='' bash scripts/dev/install-rust-tools.sh --consumer "$ROOT" \
                        --package sccache --version "$SHARED_TOOLING_SCCACHE_VERSION" --bin sccache --profile release)"
                    export CANIC_SCCACHE_BIN="$selected"
                fi
            fi
            ;;
        *)
            selected="$(command -v "$RUSTC_WRAPPER")" || {
                echo 'RUSTC_WRAPPER must select an executable (or be explicitly empty)' >&2; exit 1;
            }
            [[ -f "$selected" && -x "$selected" ]] || {
                echo 'RUSTC_WRAPPER must select an executable file' >&2; exit 1;
            }
            [[ "${selected##*/}" == sccache ]] || exit 0
            ;;
    esac
    [[ "$selected" == /* && -x "$selected" ]] || {
        echo 'Selected sccache must be an absolute executable; run make prepare-build-environment or supply CANIC_SCCACHE_BIN' >&2
        exit 1
    }
    actual="$("$selected" --version)"
    [[ "$actual" == "sccache $SHARED_TOOLING_SCCACHE_VERSION" ]] || {
        printf 'Selected sccache reports %s; policy requires %s. Prepare the managed binary or correct the explicit override.\n' "$actual" "$SHARED_TOOLING_SCCACHE_VERSION" >&2
        exit 1
    }
    if [[ "$RUSTC_WRAPPER" == "$ROOT/scripts/ci/run-canic-sccache.sh" ]]; then
        # Admit the effective runtime through its existing owner, without a server.
        bash "$RUSTC_WRAPPER" --version >/dev/null
    fi
    exit 0
fi

make --no-print-directory prepare-build-environment
if [[ "${RUSTC_WRAPPER:-}" == "$ROOT/scripts/ci/run-canic-sccache.sh" && -z "${CANIC_SCCACHE_BIN:-}" ]]; then
    # Direct tool-install Cargo children need the freshly prepared Rust selection,
    # too; a child Make's exports cannot update this preparation process.
    CANIC_SCCACHE_BIN="$(bash scripts/dev/install-rust-tools.sh --consumer "$ROOT" \
        --package sccache --version "$SHARED_TOOLING_SCCACHE_VERSION" --bin sccache --profile release --check)"
    export CANIC_SCCACHE_BIN
fi
# Preserve explicit offline policy. All consumers use the same locked graph roster.
make --no-print-directory prepare-locked-dependencies
case "$1" in
    release)
        rustup target add wasm32-unknown-unknown
        make --no-print-directory install-tools
        make --no-print-directory tools-check
        ;;
    preflight)
        make --no-print-directory install-host-tools
        make --no-print-directory install-rust-tools
        bash scripts/ci/testkit-server.sh >/dev/null
        make --no-print-directory install-release-metadata-editor
        make --no-print-directory release-metadata-editor-check
        ;;
    security)
        make --no-print-directory install-host-tools
        ;;
    checks)
        make --no-print-directory install-host-tools
        make --no-print-directory install-rust-tools
        ;;
esac
if [[ "$1" == release || "$1" == security ]]; then
    cargo install cargo-audit --version "$CANIC_CARGO_AUDIT_VERSION" --locked
fi
if [[ "$1" == release || "$1" == preflight ]]; then
    cargo install cargo-edit --version "$CANIC_CARGO_EDIT_VERSION" --locked
    bash scripts/ci/install-pinned-ci-tool.sh actionlint
    bash scripts/ci/install-pinned-ci-tool.sh shellcheck
fi
