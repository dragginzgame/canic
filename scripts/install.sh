#!/usr/bin/env bash
set -euo pipefail

CANIC_INSTALLER_VERSION="${CANIC_INSTALLER_VERSION:-0.26.4}"
CANIC_RUST_TOOLCHAIN="${CANIC_RUST_TOOLCHAIN:-1.94.1}"
RUSTUP_INIT_URL="https://sh.rustup.rs"
DFX_INSTALL_URL="https://internetcomputer.org/install.sh"
CANIC_DEV_TOOLS=(
    cargo-watch
    cargo-edit
    cargo-get
    cargo-sort
    cargo-sort-derives
)
CANIC_WASM_TOOLS=(
    candid-extractor
    ic-wasm
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
    cyan_command "cargo +$CANIC_RUST_TOOLCHAIN install --locked ${tools[*]}"
    cargo_toolchain install --locked "${tools[@]}"
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

    install_cargo_tools "Rust development tools" "${CANIC_DEV_TOOLS[@]}"
    install_cargo_tools "Wasm and Candid tools" "${CANIC_WASM_TOOLS[@]}"

    yellow "Canic installer:"
    cyan_command "cargo +$CANIC_RUST_TOOLCHAIN install --locked canic-installer --version $CANIC_INSTALLER_VERSION"
    cargo_toolchain install --locked canic-installer --version "$CANIC_INSTALLER_VERSION"

    configure_git_hooks_if_present

    if ! command -v dfx >/dev/null 2>&1; then
        echo >&2
        require_command curl
        yellow "dfx:"
        cyan_command "sh -ci \"\$(curl -fsSL $DFX_INSTALL_URL)\""
        sh -ci "$(curl -fsSL "$DFX_INSTALL_URL")"
        export PATH="$HOME/bin:$HOME/.local/bin:$HOME/.local/share/dfx/bin:$PATH"

        if command -v dfx >/dev/null 2>&1; then
            green "dfx installed: $(command -v dfx)"
        else
            yellow "dfx installed, but your current shell may need a PATH refresh before \`dfx\` is visible."
        fi
    else
        green "dfx already installed: $(command -v dfx)"
    fi

    echo >&2
    green "Canic setup complete."
}

main "$@"
