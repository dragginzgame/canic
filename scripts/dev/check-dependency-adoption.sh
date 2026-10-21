#!/usr/bin/env bash
# Focused post-update admission; never updates locks or refreshes embedded artifacts.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
mode=packages
case "${1:-}" in
    --admission-only | --fixtures-only)
        mode="${1#--}"
        shift
        [[ $# -eq 0 ]] || { echo 'release phases accept no package arguments' >&2; exit 2; }
        ;;
    '')
        echo 'usage: check-dependency-adoption.sh PACKAGE [PACKAGE ...]' >&2
        exit 2
        ;;
esac
packages=()
for package in "$@"; do
    [[ "$package" =~ ^[A-Za-z0-9_][A-Za-z0-9_-]*$ ]] || {
        echo "invalid package name: $package" >&2; exit 2;
    }
    packages+=(--package "$package")
done
manifests=(Cargo.toml integrations/blob-service/consumer/Cargo.toml integrations/blob-service/embedded-consumer/Cargo.toml)
roots=(canic-blob-service blob-service-consumer blob-service-embedded-consumer)
if [[ "$mode" != fixtures-only ]]; then
    for index in "${!manifests[@]}"; do
        manifest="${manifests[$index]}"
        echo "==> locked dependency admission: $manifest"
        cargo metadata --locked --offline --format-version 1 --manifest-path "$manifest" > /dev/null
        bash scripts/ci/check-cargo-dependency-boundaries.sh --consumer "$ROOT/$(dirname "$manifest")"
        # Normal Wasm dependencies only: host build tools and dev dependencies are
        # separate graphs. Cargo owns package identities, aliases and target filtering.
        tree="$(cargo tree --locked --offline --manifest-path "$manifest" \
            --package "${roots[$index]}" --target wasm32-unknown-unknown \
            --edges normal --prefix none --format '{p}')"
        for owner in ic-memory ic-platform ic-storage; do
            identities="$(printf '%s\n' "$tree" | awk -v owner="$owner" \
                '$1 == owner { sub(/ \(\*\)$/, ""); print }' | LC_ALL=C sort -u)"
            count="$(printf '%s\n' "$identities" | awk 'NF { count++ } END { print count+0 }')"
            if (( count > 1 )); then
                printf 'Conflicting %s identities in %s:\n%s\n' "$owner" "$manifest" "$identities" >&2
                cargo tree --locked --offline --manifest-path "$manifest" \
                    --package "${roots[$index]}" --target wasm32-unknown-unknown --edges normal >&2
                echo 'Align the owning published dependencies; do not bridge runtime handles.' >&2
                exit 1
            fi
        done
    done
fi
[[ "$mode" != admission-only ]] || exit 0
if [[ "$mode" == packages ]]; then
    echo '==> affected root packages: Clippy'
    cargo clippy --locked --offline "${packages[@]}" --all-targets --all-features -- -D warnings
fi
for manifest in "${manifests[@]:1}"; do
    echo "==> fixture Clippy: $manifest"
    cargo clippy --locked --offline --manifest-path "$manifest" --all-targets --all-features -- -D warnings
done
echo '==> embedded allocation peer qualification (read-only)'
cargo run --locked --offline -p canic-testing-internal --example verify_embedded_root
echo 'Dependency adoption checks passed; managed Wasm/install proofs and release validation remain separate.'
