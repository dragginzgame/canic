#!/usr/bin/env bash
# Run the incident-specific upgrade/restoration proof on an owned local server.
set -euo pipefail
if [[ $# != 2 ]]; then
    echo "usage: $0 READ_ONLY_TOKO_WORKSPACE PREPARED_REPAIR_DIRECTORY" >&2
    exit 2
fi
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd -- "$script_dir/../../.." && pwd)
workspace=$(realpath -- "$1")
repair=$(realpath -- "$2")
case "$repair" in
    "$repo_root"/target/canic188-*) ;;
    *) echo "Repair output must be inside Canic target/canic188-*" >&2; exit 2 ;;
esac
cd -- "$repo_root"
flock -n target/debug/.cargo-lock -c true
bash scripts/ci/run-with-test-scratch.sh cargo build --locked -p canic-host \
    --features local-fleet --example canic188_qualify

server_dir=$(mktemp -d "$repair/pocketic.XXXXXXXX")
server="${XDG_CACHE_HOME:-$HOME/.cache}/canic/pocket-ic-server-16.0.0/pocket-ic"
"$server" --port-file "$server_dir/port" --ttl 600 --hard-ttl 600 \
    > "$repair/pocketic-server.log" 2>&1 &
server_pid=$!
cleanup() {
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
    rm -rf -- "$server_dir"
}
trap cleanup EXIT
for ((attempt=0; attempt<100; attempt++)); do
    if [[ -s "$server_dir/port" ]]; then break; fi
    kill -0 "$server_pid"
    sleep 0.1
done
CANIC_POCKET_IC_SERVER_URL="http://127.0.0.1:$(cat "$server_dir/port")/"
export CANIC_POCKET_IC_SERVER_URL
target/debug/examples/canic188_qualify "$workspace" "$repair"
