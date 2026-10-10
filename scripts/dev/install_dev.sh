#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=/dev/null
source "$ROOT_DIR/tool-versions.env"
CANIC_CLI_VERSION="${CANIC_CLI_VERSION:-0.111.0}"
CANIC_RUST_TOOLCHAIN="${CANIC_RUST_TOOLCHAIN:-1.99.0}"
ACTIONLINT_INSTALL_DIR="${ACTIONLINT_INSTALL_DIR:-$HOME/.local/bin}"
SHELLCHECK_INSTALL_DIR="${SHELLCHECK_INSTALL_DIR:-$HOME/.local/bin}"
CANIC_DEV_TOOLS=(
    "cargo-watch@$CANIC_CARGO_WATCH_VERSION"
    "cargo-edit@$CANIC_CARGO_EDIT_VERSION"
    "sccache@$CANIC_SCCACHE_VERSION"
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
    cyan_command "ACTIONLINT_INSTALL_DIR=$ACTIONLINT_INSTALL_DIR bash scripts/ci/install-pinned-ci-tool.sh actionlint"
    require_command curl
    require_command tar
    bin="$(
        ACTIONLINT_INSTALL_DIR="$ACTIONLINT_INSTALL_DIR" \
            bash "$ROOT_DIR/scripts/ci/install-pinned-ci-tool.sh" actionlint
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
    cyan_command "SHELLCHECK_INSTALL_DIR=$SHELLCHECK_INSTALL_DIR bash scripts/ci/install-pinned-ci-tool.sh shellcheck"
    require_command curl
    require_command tar
    bin="$(
        SHELLCHECK_INSTALL_DIR="$SHELLCHECK_INSTALL_DIR" \
            bash "$ROOT_DIR/scripts/ci/install-pinned-ci-tool.sh" shellcheck
    )"

    green "ShellCheck installed: $("$bin" --version 2>&1 | head -n 1)"
    if command -v shellcheck >/dev/null 2>&1; then
        green "shellcheck on PATH: $(command -v shellcheck)"
    else
        yellow "shellcheck installed at $SHELLCHECK_INSTALL_DIR/shellcheck; add $SHELLCHECK_INSTALL_DIR to PATH to run it directly."
    fi
}

install_repository_tools() {
    yellow "Repository JSON/YAML and IC tools:"
    make -C "$ROOT_DIR" --no-print-directory install-tools
    export PATH="$ROOT_DIR/.tools/host/bin:$ROOT_DIR/.tools/ic/bin:$ROOT_DIR/.tools/rust/bin:$PATH"
    hash -r 2>/dev/null || true
}

require_python() {
    yellow "Python 3:"
    require_command python3
    green "python3 ready: $(python3 --version 2>&1)"
}

configure_git_formatting_hook_if_present() {
    local hook_installer="$ROOT_DIR/scripts/dev/install-git-hooks.sh"
    local pre_commit_hook="$ROOT_DIR/.githooks/pre-commit"

    if [ ! -f "$hook_installer" ] || [ ! -f "$pre_commit_hook" ]; then
        return 0
    fi

    yellow "Git formatting hook:"
    cyan_command "bash scripts/dev/install-git-hooks.sh"
    bash "$hook_installer"
}

main() {
    if [ "${1:-}" = "--update-prereqs" ]; then
        blue "Checking Python, shell lint, workflow lint, ICP CLI, and Wasm prerequisites"
        require_python
        install_or_update_shellcheck
        install_or_update_actionlint
        install_repository_tools
        green "Python, shell lint, workflow lint, ICP CLI, and Wasm prerequisites ready."
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
    require_command sccache
    green "sccache ready: $(sccache --version 2>&1)"
    install_or_update_shellcheck
    install_or_update_actionlint
    install_repository_tools

    yellow "Canic CLI:"
    cyan_command "cargo +$CANIC_RUST_TOOLCHAIN install --quiet --locked canic-cli --version $CANIC_CLI_VERSION"
    cargo_toolchain install --quiet --locked canic-cli --version "$CANIC_CLI_VERSION"

    configure_git_formatting_hook_if_present

    echo >&2
    green "Canic setup complete."
    if command -v canic >/dev/null 2>&1; then
        green "canic ready: $(command -v canic)"
    else
        yellow "canic installed under Cargo's bin directory; add \$HOME/.cargo/bin to PATH before running it."
    fi
}

main "$@"
