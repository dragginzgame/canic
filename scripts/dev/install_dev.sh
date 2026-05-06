#!/usr/bin/env bash
set -euo pipefail

CANIC_INSTALLER_VERSION="${CANIC_INSTALLER_VERSION:-0.31.0}"
CANIC_CLI_VERSION="${CANIC_CLI_VERSION:-$CANIC_INSTALLER_VERSION}"
CANIC_RUST_TOOLCHAIN="${CANIC_RUST_TOOLCHAIN:-1.95.0}"
ACTIONLINT_VERSION="${ACTIONLINT_VERSION:-1.7.8}"
ACTIONLINT_INSTALL_DIR="${ACTIONLINT_INSTALL_DIR:-$HOME/.local/bin}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
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
CANIC_PYTHON_PACKAGE_BREW="${CANIC_PYTHON_PACKAGE_BREW:-python}"
CANIC_PYTHON_PACKAGE_APT="${CANIC_PYTHON_PACKAGE_APT:-python3}"
CANIC_PYTHON_PACKAGE_DNF="${CANIC_PYTHON_PACKAGE_DNF:-python3}"
CANIC_PYTHON_PACKAGE_YUM="${CANIC_PYTHON_PACKAGE_YUM:-python3}"
CANIC_PYTHON_PACKAGE_PACMAN="${CANIC_PYTHON_PACKAGE_PACMAN:-python}"
CANIC_PYTHON_PACKAGE_ZYPPER="${CANIC_PYTHON_PACKAGE_ZYPPER:-python3}"

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

run_with_optional_sudo() {
    if [ "$(id -u)" -eq 0 ]; then
        "$@"
    elif command -v sudo >/dev/null 2>&1; then
        sudo "$@"
    else
        red "missing required privilege helper: sudo"
        exit 1
    fi
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

detect_python_package_manager() {
    if command -v brew >/dev/null 2>&1; then
        printf 'brew\n'
    elif command -v apt-get >/dev/null 2>&1; then
        printf 'apt-get\n'
    elif command -v dnf >/dev/null 2>&1; then
        printf 'dnf\n'
    elif command -v yum >/dev/null 2>&1; then
        printf 'yum\n'
    elif command -v pacman >/dev/null 2>&1; then
        printf 'pacman\n'
    elif command -v zypper >/dev/null 2>&1; then
        printf 'zypper\n'
    else
        printf 'none\n'
    fi
}

install_or_update_python() {
    local mode="$1"
    local manager

    if command -v python3 >/dev/null 2>&1; then
        green "python3 already installed: $(command -v python3)"
        if [ "$mode" = "install" ]; then
            return 0
        fi
    fi

    manager="$(detect_python_package_manager)"
    if [ "$manager" = "none" ]; then
        red "unable to install python3 automatically: no supported package manager found"
        exit 1
    fi

    yellow "Python 3:"

    case "$manager" in
    brew)
        if [ "$mode" = "update" ]; then
            cyan_command "brew upgrade $CANIC_PYTHON_PACKAGE_BREW || brew install $CANIC_PYTHON_PACKAGE_BREW"
            brew upgrade "$CANIC_PYTHON_PACKAGE_BREW" || brew install "$CANIC_PYTHON_PACKAGE_BREW"
        else
            cyan_command "brew install $CANIC_PYTHON_PACKAGE_BREW"
            brew install "$CANIC_PYTHON_PACKAGE_BREW"
        fi
        ;;
    apt-get)
        cyan_command "sudo apt-get update"
        run_with_optional_sudo apt-get update
        cyan_command "sudo apt-get install -y $CANIC_PYTHON_PACKAGE_APT"
        run_with_optional_sudo apt-get install -y "$CANIC_PYTHON_PACKAGE_APT"
        ;;
    dnf)
        cyan_command "sudo dnf install -y $CANIC_PYTHON_PACKAGE_DNF"
        run_with_optional_sudo dnf install -y "$CANIC_PYTHON_PACKAGE_DNF"
        ;;
    yum)
        cyan_command "sudo yum install -y $CANIC_PYTHON_PACKAGE_YUM"
        run_with_optional_sudo yum install -y "$CANIC_PYTHON_PACKAGE_YUM"
        ;;
    pacman)
        cyan_command "sudo pacman -Sy --needed $CANIC_PYTHON_PACKAGE_PACMAN"
        run_with_optional_sudo pacman -Sy --needed "$CANIC_PYTHON_PACKAGE_PACMAN"
        ;;
    zypper)
        cyan_command "sudo zypper install -y $CANIC_PYTHON_PACKAGE_ZYPPER"
        run_with_optional_sudo zypper install -y "$CANIC_PYTHON_PACKAGE_ZYPPER"
        ;;
    esac

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
    if [ "${1:-}" = "--update-python" ]; then
        blue "Updating Python and workflow lint prerequisites"
        install_or_update_python update
        install_or_update_actionlint
        green "Python and workflow lint update complete."
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

    install_or_update_python install

    install_cargo_tools "Rust development tools" "${CANIC_DEV_TOOLS[@]}"
    install_cargo_tools "Wasm and Candid tools" "${CANIC_WASM_TOOLS[@]}"
    install_or_update_actionlint

    yellow "Canic installer:"
    cyan_command "cargo +$CANIC_RUST_TOOLCHAIN install --locked canic-installer --version $CANIC_INSTALLER_VERSION"
    cargo_toolchain install --locked canic-installer --version "$CANIC_INSTALLER_VERSION"

    yellow "Canic CLI:"
    cyan_command "cargo +$CANIC_RUST_TOOLCHAIN install --locked canic-cli --version $CANIC_CLI_VERSION"
    cargo_toolchain install --locked canic-cli --version "$CANIC_CLI_VERSION"

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
    if command -v canic >/dev/null 2>&1; then
        green "canic ready: $(command -v canic)"
    else
        yellow "canic installed under Cargo's bin directory; add \$HOME/.cargo/bin to PATH before running it."
    fi
}

main "$@"
