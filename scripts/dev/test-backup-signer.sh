#!/usr/bin/env bash
# Linux qualification in an isolated Secret Service. Recovery words never enter logs.
set -euo pipefail
umask 077
cd -- "$(dirname -- "$0")/../.."
repo=$PWD
export CANIC_BACKUP_SIGNER="$repo/target/backup-sign-tools/bin/ic-sign"
[[ -x "$CANIC_BACKUP_SIGNER" ]] || { echo 'Run make backup-signer-setup first.' >&2; exit 1; }
mkdir -p target/backup
canic artifact build --package canic-root --features backup --output target/backup/managed-root.wasm --json > target/backup/managed-root.json
canic artifact build --package canic-demo --output target/backup/managed-counter.wasm --json > target/backup/managed-counter.json
export CANIC_BACKUP_FIXTURE
CANIC_BACKUP_FIXTURE=$(mktemp -d)
trap 'rm -rf -- "$CANIC_BACKUP_FIXTURE"' EXIT
export ICP_HOME="$CANIC_BACKUP_FIXTURE" XDG_CONFIG_HOME="$CANIC_BACKUP_FIXTURE/config" XDG_DATA_HOME="$CANIC_BACKUP_FIXTURE/data" XDG_RUNTIME_DIR="$CANIC_BACKUP_FIXTURE/runtime"
unset GNOME_KEYRING_CONTROL DISPLAY WAYLAND_DISPLAY XAUTHORITY
mkdir -p "$CANIC_BACKUP_FIXTURE/store" "$XDG_CONFIG_HOME" "$XDG_DATA_HOME" "$XDG_RUNTIME_DIR"
dbus-run-session -- bash -seu -o pipefail <<'SESSION'
    printf "%s\n" disposable-fixture | gnome-keyring-daemon --foreground --unlock --components=secrets --control-directory "$XDG_RUNTIME_DIR/control" >/dev/null 2>&1 &
    daemon=$!
    trap 'kill -KILL "$daemon" 2>/dev/null || true; wait "$daemon" 2>/dev/null || true' EXIT
    for ((i=0; i<100; i++)); do
        if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus --method org.freedesktop.DBus.NameHasOwner org.freedesktop.secrets | grep -q true; then break; fi
        sleep 0.05
    done
    gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus --method org.freedesktop.DBus.NameHasOwner org.freedesktop.secrets | grep -q true
    icp identity new canic-backup-test --storage keyring --json >/dev/null 2>&1
    cargo test --locked --offline --manifest-path qualification/Cargo.toml -p canic-qualification --features backup --test backup -- --nocapture
SESSION
