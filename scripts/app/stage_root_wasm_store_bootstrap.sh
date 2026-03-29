#!/usr/bin/env bash

set -euo pipefail

ROOT_CANISTER="${1:-root}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
WASM_PATH="${WASM_STORE_WASM:-${ROOT_DIR}/.dfx/local/canisters/wasm_store/wasm_store.wasm.gz}"
CHUNK_BYTES="${WASM_STORE_BOOTSTRAP_CHUNK_BYTES:-1000000}"
TMP_STAGE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/canic-ws-bootstrap.XXXXXX")"

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

if [ ! -f "${WASM_PATH}" ]; then
    echo "missing wasm_store wasm at ${WASM_PATH}" >&2
    exit 1
fi

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

python3 - "${WASM_PATH}" "${CANIC_VERSION}" "${CHUNK_BYTES}" "${TMP_STAGE_DIR}" <<'PY'
import hashlib
import pathlib
import sys

wasm_path = pathlib.Path(sys.argv[1])
version = sys.argv[2]
chunk_bytes = int(sys.argv[3])
out_dir = pathlib.Path(sys.argv[4])
wasm = wasm_path.read_bytes()

template_id = "embedded:wasm_store"
role = "wasm_store"
binding = "bootstrap"
payload_hash = hashlib.sha256(wasm).digest()
chunks = [wasm[i : i + chunk_bytes] for i in range(0, len(wasm), chunk_bytes)]
chunk_hashes = [hashlib.sha256(chunk).digest() for chunk in chunks]

def blob_literal(data: bytes) -> str:
    return 'blob "' + "".join(f"\\{byte:02x}" for byte in data) + '"'

manifest = f'''(
  record {{
    template_id = "{template_id}";
    role = "{role}";
    version = "{version}";
    payload_hash = {blob_literal(payload_hash)};
    payload_size_bytes = {len(wasm)} : nat64;
    store_binding = "{binding}";
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

(out_dir / "manifest.did").write_text(manifest, encoding="utf-8")
(out_dir / "prepare.did").write_text(prepare, encoding="utf-8")

for index, chunk in enumerate(chunks):
    chunk_arg = f'''(
  record {{
    template_id = "{template_id}";
    version = "{version}";
    chunk_index = {index} : nat32;
    bytes = {blob_literal(chunk)};
  }}
)'''
    (out_dir / f"chunk-{index:06d}.did").write_text(chunk_arg, encoding="utf-8")
PY

call_root_method() {
    local preferred="$1"
    local fallback="$2"
    local arg_file="${3:-}"
    local output

    if [ -n "${arg_file}" ]; then
        if output="$(dfx canister call "${ROOT_CANISTER}" "${preferred}" --argument-file "${arg_file}" 2>&1)"; then
            printf '%s\n' "${output}"
            return 0
        fi
    else
        if output="$(dfx canister call "${ROOT_CANISTER}" "${preferred}" '()' 2>&1)"; then
            printf '%s\n' "${output}"
            return 0
        fi
    fi

    if printf '%s' "${output}" | grep -qiE "method.*not found|has no method|did not find method|unknown method"; then
        if [ -n "${arg_file}" ]; then
            dfx canister call "${ROOT_CANISTER}" "${fallback}" --argument-file "${arg_file}"
        else
            dfx canister call "${ROOT_CANISTER}" "${fallback}" '()'
        fi
        return 0
    fi

    printf '%s\n' "${output}" >&2
    return 1
}

echo "Staging gzipped wasm_store bootstrap into ${ROOT_CANISTER} from ${WASM_PATH}"
echo "Bootstrap payload size: $(format_bytes "$(stat -c '%s' "${WASM_PATH}")")"

call_root_method \
    "canic_wasm_store_bootstrap_stage_manifest_admin" \
    "wasm_store_bootstrap_stage_manifest_admin" \
    "${TMP_STAGE_DIR}/manifest.did" >/dev/null

call_root_method \
    "canic_wasm_store_bootstrap_prepare_admin" \
    "wasm_store_bootstrap_prepare_admin" \
    "${TMP_STAGE_DIR}/prepare.did" >/dev/null

while IFS= read -r chunk_file; do
    call_root_method \
        "canic_wasm_store_bootstrap_publish_chunk_admin" \
        "wasm_store_bootstrap_publish_chunk_admin" \
        "${chunk_file}" >/dev/null
done < <(find "${TMP_STAGE_DIR}" -name 'chunk-*.did' | sort)

call_root_method \
    "canic_wasm_store_bootstrap_resume_root_admin" \
    "wasm_store_bootstrap_resume_root_admin" >/dev/null

echo "Resumed root bootstrap after staged wasm_store upload"
