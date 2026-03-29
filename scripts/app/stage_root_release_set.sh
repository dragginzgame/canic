#!/usr/bin/env bash

set -euo pipefail

ROOT_CANISTER="${1:-root}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
CONFIG_PATH="${CANIC_CONFIG_PATH:-${ROOT_DIR}/crates/canisters/canic.toml}"
WASM_ROOT="${CANIC_STAGE_WASM_DIR:-${ROOT_DIR}/.dfx/local/canisters}"
CHUNK_BYTES="${CANIC_TEMPLATE_STAGE_CHUNK_BYTES:-1048576}"
TMP_STAGE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/canic-release-stage.XXXXXX")"

cleanup() {
    rm -rf "${TMP_STAGE_DIR}"
}

trap cleanup EXIT

format_bytes() {
    python3 - "$1" <<'PY'
import sys

value = float(sys.argv[1])
units = ["B", "KiB", "MiB", "GiB", "TiB"]
unit = 0

while value >= 1024.0 and unit < len(units) - 1:
    value /= 1024.0
    unit += 1

print(f"{value:.2f} {units[unit]}")
PY
}

CANIC_VERSION="$(
    python3 - "${ROOT_DIR}/Cargo.toml" <<'PY'
import pathlib
import sys
import tomllib

path = pathlib.Path(sys.argv[1])
with path.open("rb") as fh:
    cargo = tomllib.load(fh)

print(cargo["workspace"]["package"]["version"])
PY
)"

python3 - "${CONFIG_PATH}" "${WASM_ROOT}" "${CANIC_VERSION}" "${CHUNK_BYTES}" "${TMP_STAGE_DIR}" <<'PY'
import hashlib
import pathlib
import sys
import tomllib

config_path = pathlib.Path(sys.argv[1])
wasm_root = pathlib.Path(sys.argv[2])
version = sys.argv[3]
chunk_bytes = int(sys.argv[4])
out_dir = pathlib.Path(sys.argv[5])

with config_path.open("rb") as fh:
    config = tomllib.load(fh)

roles = set()
for subnet in config.get("subnets", {}).values():
    for role in subnet.get("canisters", {}).keys():
        if role != "root":
            roles.add(role)

# The first wasm_store is still the bootstrap exception: it is not part of the
# ordinary config-defined release set because the store does not exist yet, but
# root still needs a staged bootstrap payload for it.
roles.add("wasm_store")

def blob_literal(data: bytes) -> str:
    return 'blob "' + "".join(f"\\{byte:02x}" for byte in data) + '"'

for role in sorted(roles):
    wasm_path = wasm_root / role / f"{role}.wasm.gz"
    if not wasm_path.is_file():
        raise SystemExit(f"missing wasm artifact for role '{role}' at {wasm_path}")

    wasm = wasm_path.read_bytes()
    payload_hash = hashlib.sha256(wasm).digest()
    chunks = [wasm[i : i + chunk_bytes] for i in range(0, len(wasm), chunk_bytes)]
    chunk_hashes = [hashlib.sha256(chunk).digest() for chunk in chunks]
    template_id = f"embedded:{role}"

    role_dir = out_dir / role
    role_dir.mkdir(parents=True, exist_ok=True)

    manifest = f'''(
  record {{
    template_id = "{template_id}";
    role = "{role}";
    version = "{version}";
    payload_hash = {blob_literal(payload_hash)};
    payload_size_bytes = {len(wasm)} : nat64;
    store_binding = "bootstrap";
    chunking_mode = variant {{ Chunked }};
    manifest_state = variant {{ Approved }};
    approved_at = null;
    created_at = 0 : nat64;
  }}
)'''

    prepare = f'''(
  record {{
    template_id = "{template_id}";
    version = "{version}";
    payload_hash = {blob_literal(payload_hash)};
    payload_size_bytes = {len(wasm)} : nat64;
    chunk_hashes = vec {{
      {"; ".join(blob_literal(chunk_hash) for chunk_hash in chunk_hashes)}
    }};
  }}
)'''

    (role_dir / "manifest.did").write_text(manifest, encoding="utf-8")
    (role_dir / "prepare.did").write_text(prepare, encoding="utf-8")

    for index, chunk in enumerate(chunks):
        chunk_arg = f'''(
  record {{
    template_id = "{template_id}";
    version = "{version}";
    chunk_index = {index} : nat32;
    bytes = {blob_literal(chunk)};
  }}
)'''
        (role_dir / f"chunk-{index:06d}.did").write_text(chunk_arg, encoding="utf-8")
PY

call_root_method() {
    local method="$1"
    local arg_file="${2:-}"

    if [ -n "${arg_file}" ]; then
        dfx canister call "${ROOT_CANISTER}" "${method}" --argument-file "${arg_file}" >/dev/null
    else
        dfx canister call "${ROOT_CANISTER}" "${method}" '()' >/dev/null
    fi
}

echo "Submitting config-driven release set to ${ROOT_CANISTER} for wasm_store publication from ${CONFIG_PATH}"
echo "Including bootstrap exception role 'wasm_store' ahead of the config-defined release set"

stage_role_dir() {
    local role_dir="$1"
    local artifact_path
    local payload_size_bytes
    local payload_size
    local chunk_count

    role="$(basename "${role_dir}")"
    artifact_path="${WASM_ROOT}/${role}/${role}.wasm.gz"
    payload_size_bytes="$(stat -c '%s' "${artifact_path}")"
    payload_size="$(format_bytes "${payload_size_bytes}")"
    chunk_count="$(find "${role_dir}" -name 'chunk-*.did' | wc -l | tr -d ' ')"

    echo "Submitting role '${role}' to ${ROOT_CANISTER} for wasm_store publication (${payload_size}, ${chunk_count} chunks)"

    if [ "${role}" = "wasm_store" ]; then
        call_root_method "canic_wasm_store_bootstrap_stage_manifest_admin" "${role_dir}/manifest.did"
        call_root_method "canic_wasm_store_bootstrap_prepare_admin" "${role_dir}/prepare.did"

        while IFS= read -r chunk_file; do
            call_root_method "canic_wasm_store_bootstrap_publish_chunk_admin" "${chunk_file}"
        done < <(find "${role_dir}" -name 'chunk-*.did' | sort)

        return
    fi

    call_root_method "canic_template_stage_manifest_admin" "${role_dir}/manifest.did"
    call_root_method "canic_template_prepare_admin" "${role_dir}/prepare.did"

    while IFS= read -r chunk_file; do
        call_root_method "canic_template_publish_chunk_admin" "${chunk_file}"
    done < <(find "${role_dir}" -name 'chunk-*.did' | sort)
}

WASM_STORE_ROLE_DIR="${TMP_STAGE_DIR}/wasm_store"
if [ -d "${WASM_STORE_ROLE_DIR}" ]; then
    echo "Submitting bootstrap role first so root can leave bootstrap wait state"
    stage_role_dir "${WASM_STORE_ROLE_DIR}"
fi

while IFS= read -r role_dir; do
    [ "${role_dir}" = "${WASM_STORE_ROLE_DIR}" ] && continue
    stage_role_dir "${role_dir}"
done < <(find "${TMP_STAGE_DIR}" -mindepth 1 -maxdepth 1 -type d | sort)

echo "Resuming root bootstrap after full release set staging"
call_root_method "canic_wasm_store_bootstrap_resume_root_admin"

echo "Waiting for ${ROOT_CANISTER} to reach READY"
attempt=0
for _ in $(seq 1 300); do
    if dfx canister call "${ROOT_CANISTER}" canic_ready '()' 2>/dev/null | grep -q "true"; then
        echo "Root bootstrap resumed and reached READY"
        exit 0
    fi
    attempt=$((attempt + 1))
    if [ $((attempt % 10)) -eq 0 ]; then
        echo "Still waiting for ${ROOT_CANISTER} to reach READY; debug with:"
        echo "  dfx canister call ${ROOT_CANISTER} canic_wasm_store_bootstrap_debug '()'"
        echo "  dfx canister call ${ROOT_CANISTER} canic_wasm_store_overview '()'"
    fi
    sleep 1
done

echo "Root bootstrap resumed but did not reach READY within 300s" >&2
echo "Bootstrap debug:" >&2
dfx canister call "${ROOT_CANISTER}" canic_wasm_store_bootstrap_debug '()' || true
echo "Wasm store overview:" >&2
dfx canister call "${ROOT_CANISTER}" canic_wasm_store_overview '()' || true
exit 1
