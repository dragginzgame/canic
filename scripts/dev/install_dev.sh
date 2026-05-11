#!/usr/bin/env bash
set -euo pipefail

CANIC_CLI_VERSION="${CANIC_CLI_VERSION:-0.34.2}"
CANIC_RUST_TOOLCHAIN="${CANIC_RUST_TOOLCHAIN:-1.95.0}"
ACTIONLINT_VERSION="${ACTIONLINT_VERSION:-1.7.8}"
ACTIONLINT_INSTALL_DIR="${ACTIONLINT_INSTALL_DIR:-$HOME/.local/bin}"
CANIC_NPM_PREFIX="${CANIC_NPM_PREFIX:-$HOME/.local}"
ICP_CLI_VERSION="${ICP_CLI_VERSION:-0.2.5}"
ICP_WASM_VERSION="${ICP_WASM_VERSION:-0.9.10}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
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
    cyan_command "ACTIONLINT_INSTALL_DIR=$ACTIONLINT_INSTALL_DIR bash scripts/ci/install-actionlint.sh $ACTIONLINT_VERSION"
    require_command curl
    require_command tar
    bin="$(ACTIONLINT_INSTALL_DIR="$ACTIONLINT_INSTALL_DIR" bash "$ROOT_DIR/scripts/ci/install-actionlint.sh" "$ACTIONLINT_VERSION")"

    green "actionlint installed: $("$bin" -version 2>&1)"
    if command -v actionlint >/dev/null 2>&1; then
        green "actionlint on PATH: $(command -v actionlint)"
    else
        yellow "actionlint installed at $ACTIONLINT_INSTALL_DIR/actionlint; add $ACTIONLINT_INSTALL_DIR to PATH to run it directly."
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

resolve_icp_npm_versions() {
    local latest_icp_cli
    local latest_ic_wasm
    local check_updates="${CANIC_CHECK_ICP_UPDATES:-0}"
    local auto_bump="${CANIC_AUTO_BUMP_ICP_TOOLS:-0}"

    if [ "$check_updates" != "1" ] && [ "$auto_bump" != "1" ]; then
        return 0
    fi

    yellow "ICP CLI update check:"
    require_command npm

    if ! latest_icp_cli="$(npm view @icp-sdk/icp-cli version 2>/dev/null)"; then
        yellow "Could not check @icp-sdk/icp-cli latest version; continuing with pinned $ICP_CLI_VERSION."
    elif [ "$latest_icp_cli" != "$ICP_CLI_VERSION" ]; then
        if [ "$auto_bump" = "1" ]; then
            yellow "@icp-sdk/icp-cli update available: pinned $ICP_CLI_VERSION, using latest $latest_icp_cli for this update."
            ICP_CLI_VERSION="$latest_icp_cli"
        else
            yellow "@icp-sdk/icp-cli update available: pinned $ICP_CLI_VERSION, latest $latest_icp_cli."
        fi
    else
        green "@icp-sdk/icp-cli is current at pinned $ICP_CLI_VERSION."
    fi

    if ! latest_ic_wasm="$(npm view @icp-sdk/ic-wasm version 2>/dev/null)"; then
        yellow "Could not check @icp-sdk/ic-wasm latest version; continuing with pinned $ICP_WASM_VERSION."
    elif [ "$latest_ic_wasm" != "$ICP_WASM_VERSION" ]; then
        if [ "$auto_bump" = "1" ]; then
            yellow "@icp-sdk/ic-wasm update available: pinned $ICP_WASM_VERSION, using latest $latest_ic_wasm for this update."
            ICP_WASM_VERSION="$latest_ic_wasm"
        else
            yellow "@icp-sdk/ic-wasm update available: pinned $ICP_WASM_VERSION, latest $latest_ic_wasm."
        fi
    else
        green "@icp-sdk/ic-wasm is current at pinned $ICP_WASM_VERSION."
    fi
}

install_or_update_icp_cli() {
    local npm_bin_dir="$CANIC_NPM_PREFIX/bin"
    local path_had_npm_bin=0

    if [[ ":$PATH:" == *":$npm_bin_dir:"* ]]; then
        path_had_npm_bin=1
    fi

    yellow "ICP CLI:"
    require_command npm
    resolve_icp_npm_versions
    mkdir -p "$npm_bin_dir"
    clean_icp_npm_staging_dirs
    export PATH="$npm_bin_dir:$PATH"
    cyan_command "npm install -g --prefix $CANIC_NPM_PREFIX @icp-sdk/icp-cli@$ICP_CLI_VERSION @icp-sdk/ic-wasm@$ICP_WASM_VERSION"
    npm install -g --prefix "$CANIC_NPM_PREFIX" "@icp-sdk/icp-cli@$ICP_CLI_VERSION" "@icp-sdk/ic-wasm@$ICP_WASM_VERSION"
    require_command icp
    require_command ic-wasm
    green "icp ready: $(icp --version 2>&1)"
    green "ic-wasm ready: $(ic-wasm --version 2>&1)"
    if [ "$path_had_npm_bin" -eq 0 ]; then
        yellow "ICP CLI tools installed under $npm_bin_dir; add it to PATH to run them directly."
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
        blue "Checking Python, workflow lint, and ICP CLI prerequisites"
        require_python
        install_or_update_actionlint
        install_or_update_icp_cli
        green "Python, workflow lint, and ICP CLI prerequisites ready."
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
    install_or_update_actionlint
    install_or_update_icp_cli

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
