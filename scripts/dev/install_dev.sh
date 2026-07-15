#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=/dev/null
source "$ROOT_DIR/tool-versions.env"
CANIC_CLI_VERSION="${CANIC_CLI_VERSION:-0.92.10}"
CANIC_RUST_TOOLCHAIN="${CANIC_RUST_TOOLCHAIN:-1.96.0}"
ACTIONLINT_INSTALL_DIR="${ACTIONLINT_INSTALL_DIR:-$HOME/.local/bin}"
SHELLCHECK_INSTALL_DIR="${SHELLCHECK_INSTALL_DIR:-$HOME/.local/bin}"
IC_WASM_INSTALL_DIR="${IC_WASM_INSTALL_DIR:-$HOME/.local/bin}"
CANIC_DEV_TOOLS=(
    "cargo-watch@$CANIC_CARGO_WATCH_VERSION"
    "cargo-edit@$CANIC_CARGO_EDIT_VERSION"
    "cargo-get@$CANIC_CARGO_GET_VERSION"
    "cargo-sort@$CANIC_CARGO_SORT_VERSION"
    "cargo-sort-derives@$CANIC_CARGO_SORT_DERIVES_VERSION"
    "ripgrep@$CANIC_RIPGREP_VERSION"
)
CANIC_WASM_TOOLS=(
    "candid-extractor@$CANIC_CANDID_EXTRACTOR_VERSION"
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

ensure_cargo_bin_on_path() {
    local cargo_bin_dir

    cargo_bin_dir="$(resolved_cargo_bin_dir)"
    mkdir -p "$cargo_bin_dir"
    export PATH="$cargo_bin_dir:$PATH"
    hash -r 2>/dev/null || true
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
    cyan_command "ACTIONLINT_INSTALL_DIR=$ACTIONLINT_INSTALL_DIR bash scripts/ci/install-actionlint.sh"
    require_command curl
    require_command tar
    bin="$(
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
    cyan_command "SHELLCHECK_INSTALL_DIR=$SHELLCHECK_INSTALL_DIR bash scripts/ci/install-shellcheck.sh"
    require_command curl
    require_command tar
    bin="$(
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
    hash -r 2>/dev/null || true
    require_command icp
    icp_path="$(command -v icp)"
    green "icp ready: $(icp --version 2>&1) ($icp_path)"
    if [ "$icp_path" != "$cargo_bin_dir/icp" ]; then
        yellow "icp resolves to $icp_path; put $cargo_bin_dir before other bin directories in PATH."
    fi
}

install_or_update_ic_wasm() {
    local bin
    local path_had_install_dir=0

    if [[ ":$PATH:" == *":$IC_WASM_INSTALL_DIR:"* ]]; then
        path_had_install_dir=1
    fi

    yellow "ic-wasm:"
    require_command curl
    require_command tar
    mkdir -p "$IC_WASM_INSTALL_DIR"
    PATH="$(resolved_cargo_bin_dir):$IC_WASM_INSTALL_DIR:$PATH"
    export PATH
    hash -r 2>/dev/null || true
    cyan_command "IC_WASM_INSTALL_DIR=$IC_WASM_INSTALL_DIR bash scripts/ci/install-ic-wasm.sh"
    bin="$(
        IC_WASM_INSTALL_DIR="$IC_WASM_INSTALL_DIR" \
            bash "$ROOT_DIR/scripts/ci/install-ic-wasm.sh"
    )"
    green "ic-wasm ready: $("$bin" --version 2>&1)"
    if [ "$path_had_install_dir" -eq 0 ]; then
        yellow "ic-wasm installed under $IC_WASM_INSTALL_DIR; add it to PATH to run it directly."
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
        blue "Checking Python, shell lint, workflow lint, and ICP CLI prerequisites"
        require_python
        install_or_update_shellcheck
        install_or_update_actionlint
        install_or_update_icp_cli
        install_or_update_ic_wasm
        green "Python, shell lint, workflow lint, and ICP CLI prerequisites ready."
        return 0
    fi

    blue "Installing Canic prerequisites"

    require_command rustup
    require_command cargo
    ensure_cargo_bin_on_path

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
    require_command rg
    green "rg ready: $(rg --version 2>&1 | head -n 1)"
    install_cargo_tools "Wasm and Candid tools" "${CANIC_WASM_TOOLS[@]}"
    install_or_update_shellcheck
    install_or_update_actionlint
    install_or_update_icp_cli
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
