#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
if {
    [ -z "${CANIC_ACTIONLINT_VERSION:-${ACTIONLINT_VERSION:-}}" ] ||
        [ -z "${CANIC_SHELLCHECK_VERSION:-}" ] ||
        [ -z "${CANIC_ICP_CLI_VERSION:-}" ] ||
        [ -z "${CANIC_ICQ_VERSION:-${CANIC_IC_QUERY_VERSION:-}}" ]
} &&
    [ -f "$ROOT_DIR/tool-versions.env" ]; then
    # shellcheck source=tool-versions.env
    source "$ROOT_DIR/tool-versions.env"
fi
CANIC_CLI_VERSION="${CANIC_CLI_VERSION:-0.68.26}"
CANIC_RUST_TOOLCHAIN="${CANIC_RUST_TOOLCHAIN:-1.96.0}"
CANIC_ACTIONLINT_VERSION="${CANIC_ACTIONLINT_VERSION:-${ACTIONLINT_VERSION:-}}"
ACTIONLINT_INSTALL_DIR="${ACTIONLINT_INSTALL_DIR:-$HOME/.local/bin}"
CANIC_SHELLCHECK_VERSION="${CANIC_SHELLCHECK_VERSION:-}"
SHELLCHECK_INSTALL_DIR="${SHELLCHECK_INSTALL_DIR:-$HOME/.local/bin}"
CANIC_ICP_CLI_VERSION="${CANIC_ICP_CLI_VERSION:-}"
CANIC_ICQ_VERSION="${CANIC_ICQ_VERSION:-${CANIC_IC_QUERY_VERSION:-}}"
CANIC_NPM_PREFIX="${CANIC_NPM_PREFIX:-$HOME/.local}"
if [ -z "$CANIC_ACTIONLINT_VERSION" ] ||
    [ -z "$CANIC_SHELLCHECK_VERSION" ] ||
    [ -z "$CANIC_ICP_CLI_VERSION" ] ||
    [ -z "$CANIC_ICQ_VERSION" ]; then
    echo "missing external tool version pin; expected tool-versions.env or explicit environment overrides" >&2
    exit 1
fi
RUSTUP_INIT_URL="https://sh.rustup.rs"
CANIC_DEV_TOOLS=(
    cargo-watch
    cargo-edit
    cargo-get
    cargo-sort
    cargo-sort-derives
)
CANIC_WASM_TOOLS=(
    candid-extractor
)

blue() {
    printf '\033[1;34m%s\033[0m\n' "$1" >&2
}

yellow() {
    printf '\033[1;33m%s\033[0m\n' "$1" >&2
}

red() {
    printf '\033[1;31m%s\033[0m\n' "$1" >&2
}

green() {
    printf '\033[1;32m%s\033[0m\n' "$1" >&2
}

cyan_command() {
    printf '  \033[1;36m%s\033[0m\n' "$1" >&2
}

cargo_toolchain() {
    cargo +"$CANIC_RUST_TOOLCHAIN" "$@"
}

resolved_cargo_bin_dir() {
    printf '%s/bin\n' "${CARGO_HOME:-$HOME/.cargo}"
}

require_command() {
    local command_name="$1"

    if command -v "$command_name" >/dev/null 2>&1; then
        return 0
    fi

    red "missing required tool: $command_name"
    exit 1
}

install_cargo_tools() {
    local label="$1"
    shift
    local tools=("$@")

    yellow "$label:"
    cyan_command "cargo +$CANIC_RUST_TOOLCHAIN install --quiet --locked ${tools[*]}"
    cargo_toolchain install --quiet --locked "${tools[@]}"
}

install_or_update_actionlint() {
    local bin

    yellow "actionlint:"
    cyan_command "CANIC_ACTIONLINT_VERSION=$CANIC_ACTIONLINT_VERSION ACTIONLINT_INSTALL_DIR=$ACTIONLINT_INSTALL_DIR bash scripts/ci/install-actionlint.sh"
    require_command curl
    require_command tar
    bin="$(
        CANIC_ACTIONLINT_VERSION="$CANIC_ACTIONLINT_VERSION" \
            ACTIONLINT_INSTALL_DIR="$ACTIONLINT_INSTALL_DIR" \
            bash "$ROOT_DIR/scripts/ci/install-actionlint.sh"
    )"

    green "actionlint installed: $("$bin" -version 2>&1)"
    if command -v actionlint >/dev/null 2>&1; then
        green "actionlint on PATH: $(command -v actionlint)"
    else
        yellow "actionlint installed at $ACTIONLINT_INSTALL_DIR/actionlint; add $ACTIONLINT_INSTALL_DIR to PATH to run it directly."
    fi
}

install_or_update_shellcheck() {
    local bin

    yellow "ShellCheck:"
    cyan_command "CANIC_SHELLCHECK_VERSION=$CANIC_SHELLCHECK_VERSION SHELLCHECK_INSTALL_DIR=$SHELLCHECK_INSTALL_DIR bash scripts/ci/install-shellcheck.sh"
    require_command curl
    require_command tar
    bin="$(
        CANIC_SHELLCHECK_VERSION="$CANIC_SHELLCHECK_VERSION" \
            SHELLCHECK_INSTALL_DIR="$SHELLCHECK_INSTALL_DIR" \
            bash "$ROOT_DIR/scripts/ci/install-shellcheck.sh"
    )"

    green "ShellCheck installed: $("$bin" --version 2>&1 | head -n 1)"
    if command -v shellcheck >/dev/null 2>&1; then
        green "shellcheck on PATH: $(command -v shellcheck)"
    else
        yellow "shellcheck installed at $SHELLCHECK_INSTALL_DIR/shellcheck; add $SHELLCHECK_INSTALL_DIR to PATH to run it directly."
    fi
}

clean_legacy_icp_npm_cli() {
    local cargo_bin_dir
    local npm_bin_dir="$CANIC_NPM_PREFIX/bin"
    local npm_icp_bin="$npm_bin_dir/icp"
    local link_target=""

    cargo_bin_dir="$(resolved_cargo_bin_dir)"
    if [ -L "$npm_icp_bin" ]; then
        link_target="$(readlink "$npm_icp_bin" || true)"
        if [[ "$link_target" == *"@icp-sdk/icp-cli"* ]]; then
            yellow "Removing legacy npm ICP CLI wrapper:"
            if command -v npm >/dev/null 2>&1; then
                cyan_command "npm uninstall -g --prefix $CANIC_NPM_PREFIX @icp-sdk/icp-cli"
                npm uninstall -g --prefix "$CANIC_NPM_PREFIX" @icp-sdk/icp-cli >/dev/null 2>&1 || true
            fi
            if [ -L "$npm_icp_bin" ]; then
                cyan_command "rm -f $npm_icp_bin"
                rm -f "$npm_icp_bin"
            fi
        fi
    elif [ -e "$npm_icp_bin" ]; then
        yellow "Leaving non-symlink ICP binary at $npm_icp_bin; remove it manually if it shadows $cargo_bin_dir/icp."
    fi
}

clean_icp_npm_staging_dirs() {
    local npm_scope_dir="$CANIC_NPM_PREFIX/lib/node_modules/@icp-sdk"
    local staging_dirs=()
    local staging_dir

    if [ ! -d "$npm_scope_dir" ]; then
        return 0
    fi

    shopt -s nullglob
    staging_dirs=(
        "$npm_scope_dir/.icp-cli-"*
        "$npm_scope_dir/.ic-wasm-"*
    )
    shopt -u nullglob

    if [ "${#staging_dirs[@]}" -eq 0 ]; then
        return 0
    fi

    yellow "Cleaning stale ICP npm staging directories:"
    for staging_dir in "${staging_dirs[@]}"; do
        cyan_command "rm -rf $staging_dir"
        rm -rf "$staging_dir"
    done
}

install_or_update_icp_cli() {
    local cargo_bin_dir
    local icp_path=""

    cargo_bin_dir="$(resolved_cargo_bin_dir)"
    yellow "ICP CLI:"
    mkdir -p "$cargo_bin_dir"
    export PATH="$cargo_bin_dir:$PATH"
    hash -r 2>/dev/null || true
    cyan_command "bash scripts/ci/install-icp-cli.sh"
    bash "$ROOT_DIR/scripts/ci/install-icp-cli.sh"
    clean_legacy_icp_npm_cli
    hash -r 2>/dev/null || true
    require_command icp
    icp_path="$(command -v icp)"
    green "icp ready: $(icp --version 2>&1) ($icp_path)"
    if [ "$icp_path" != "$cargo_bin_dir/icp" ]; then
        yellow "icp resolves to $icp_path; put $cargo_bin_dir before other bin directories in PATH."
    fi
}

install_or_update_ic_query() {
    local cargo_bin_dir
    local icq_path=""

    cargo_bin_dir="$(resolved_cargo_bin_dir)"
    yellow "IC Query CLI:"
    mkdir -p "$cargo_bin_dir"
    export PATH="$cargo_bin_dir:$PATH"
    hash -r 2>/dev/null || true
    cyan_command "bash scripts/ci/install-ic-query.sh"
    bash "$ROOT_DIR/scripts/ci/install-ic-query.sh"
    hash -r 2>/dev/null || true
    require_command icq
    icq_path="$(command -v icq)"
    green "icq ready: $(icq --version 2>&1) ($icq_path)"
    if [ "$icq_path" != "$cargo_bin_dir/icq" ]; then
        yellow "icq resolves to $icq_path; put $cargo_bin_dir before other bin directories in PATH."
    fi
}

install_or_update_ic_wasm() {
    local npm_bin_dir="$CANIC_NPM_PREFIX/bin"
    local path_had_npm_bin=0

    if [[ ":$PATH:" == *":$npm_bin_dir:"* ]]; then
        path_had_npm_bin=1
    fi

    yellow "ic-wasm:"
    require_command npm
    mkdir -p "$npm_bin_dir"
    clean_icp_npm_staging_dirs
    PATH="$(resolved_cargo_bin_dir):$npm_bin_dir:$PATH"
    export PATH
    hash -r 2>/dev/null || true
    cyan_command "npm install -g --prefix $CANIC_NPM_PREFIX @icp-sdk/ic-wasm"
    npm install -g --prefix "$CANIC_NPM_PREFIX" @icp-sdk/ic-wasm
    require_command ic-wasm
    green "ic-wasm ready: $(ic-wasm --version 2>&1)"
    if [ "$path_had_npm_bin" -eq 0 ]; then
        yellow "ic-wasm installed under $npm_bin_dir; add it to PATH to run it directly."
    fi
}

require_python() {
    yellow "Python 3:"
    require_command python3
    green "python3 ready: $(python3 --version 2>&1)"
}

configure_git_hooks_if_present() {
    if [ -d .git ] && [ -d .githooks ]; then
        yellow "Git hooks:"
        cyan_command "git config --local core.hooksPath .githooks"
        git config --local core.hooksPath .githooks
        chmod +x .githooks/* 2>/dev/null || true
    fi
}

main() {
    if [ "${1:-}" = "--update-prereqs" ]; then
        blue "Checking Python, shell lint, workflow lint, ICP CLI, and IC query prerequisites"
        require_python
        install_or_update_shellcheck
        install_or_update_actionlint
        install_or_update_icp_cli
        install_or_update_ic_query
        install_or_update_ic_wasm
        green "Python, shell lint, workflow lint, ICP CLI, and IC query prerequisites ready."
        return 0
    fi

    blue "Installing Canic prerequisites"

    if ! command -v rustup >/dev/null 2>&1 || ! command -v cargo >/dev/null 2>&1; then
        require_command curl
        yellow "Rust bootstrap:"
        cyan_command "curl -fsSL $RUSTUP_INIT_URL | sh -s -- -y --profile minimal --default-toolchain $CANIC_RUST_TOOLCHAIN"
        curl -fsSL "$RUSTUP_INIT_URL" | sh -s -- -y --profile minimal --default-toolchain "$CANIC_RUST_TOOLCHAIN"
        export PATH="$HOME/.cargo/bin:$PATH"
    fi

    require_command rustup
    require_command cargo

    yellow "Rust toolchain:"
    cyan_command "rustup toolchain install $CANIC_RUST_TOOLCHAIN"
    rustup toolchain install "$CANIC_RUST_TOOLCHAIN"

    yellow "Rust components:"
    cyan_command "rustup component add --toolchain $CANIC_RUST_TOOLCHAIN rustfmt clippy"
    rustup component add --toolchain "$CANIC_RUST_TOOLCHAIN" rustfmt clippy

    yellow "Wasm target:"
    cyan_command "rustup target add --toolchain $CANIC_RUST_TOOLCHAIN wasm32-unknown-unknown"
    rustup target add --toolchain "$CANIC_RUST_TOOLCHAIN" wasm32-unknown-unknown

    require_python

    install_cargo_tools "Rust development tools" "${CANIC_DEV_TOOLS[@]}"
    install_cargo_tools "Wasm and Candid tools" "${CANIC_WASM_TOOLS[@]}"
    install_or_update_shellcheck
    install_or_update_actionlint
    install_or_update_icp_cli
    install_or_update_ic_query
    install_or_update_ic_wasm

    yellow "Canic CLI:"
    cyan_command "cargo +$CANIC_RUST_TOOLCHAIN install --quiet --locked canic-cli --version $CANIC_CLI_VERSION"
    cargo_toolchain install --quiet --locked canic-cli --version "$CANIC_CLI_VERSION"

    configure_git_hooks_if_present

    echo >&2
    green "Canic setup complete."
    if command -v canic >/dev/null 2>&1; then
        green "canic ready: $(command -v canic)"
    else
        yellow "canic installed under Cargo's bin directory; add \$HOME/.cargo/bin to PATH before running it."
    fi
}

main "$@"
