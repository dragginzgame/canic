#!/usr/bin/env bash

set -euo pipefail

ROOT_CANISTER="${1:-${ROOT_CANISTER:-root}}"
NETWORK="${DFX_NETWORK:-local}"
CHUNK_BYTES="${CANIC_TEMPLATE_STAGE_CHUNK_BYTES:-1048576}"

require_dfx() {
    if ! command -v dfx >/dev/null 2>&1; then
        echo "dfx is required for root bootstrap staging" >&2
        exit 1
    fi
}

discover_project_root() {
    python3 - "${CANIC_CONFIG_PATH:-}" "$PWD" <<'PY'
import pathlib
import sys

config_arg = sys.argv[1]
cwd = pathlib.Path(sys.argv[2]).resolve()
starts = []

if config_arg:
    config_path = pathlib.Path(config_arg)
    if not config_path.is_absolute():
        config_path = (cwd / config_path).resolve()
    starts.append(config_path.parent)

starts.append(cwd)

seen = set()
fallback = None

for start in starts:
    for candidate in (start, *start.parents):
        if candidate in seen:
            continue
        seen.add(candidate)

        if (candidate / "dfx.json").is_file():
            print(candidate)
            raise SystemExit(0)

        if fallback is None and (candidate / "Cargo.toml").is_file():
            fallback = candidate

if fallback is not None:
    print(fallback)
else:
    print(cwd)
PY
}

discover_config_path() {
    python3 - "$PROJECT_ROOT" "${CANIC_CONFIG_PATH:-}" <<'PY'
import pathlib
import sys

project_root = pathlib.Path(sys.argv[1]).resolve()
config_arg = sys.argv[2]

if config_arg:
    config_path = pathlib.Path(config_arg)
    if not config_path.is_absolute():
        config_path = (project_root / config_path).resolve()
    print(config_path)
    raise SystemExit(0)

for candidate in (
    project_root / "canic.toml",
    project_root / "canisters" / "canic.toml",
):
    if candidate.is_file():
        print(candidate)
        raise SystemExit(0)

raise SystemExit(
    "unable to locate canic config; set CANIC_CONFIG_PATH to your project canic.toml"
)
PY
}

discover_stage_version() {
    python3 - "$PROJECT_ROOT" "${CANIC_TEMPLATE_STAGE_VERSION:-}" <<'PY'
import pathlib
import sys
import tomllib

project_root = pathlib.Path(sys.argv[1]).resolve()
explicit = sys.argv[2]

if explicit:
    print(explicit)
    raise SystemExit(0)

for candidate in (
    project_root / "Cargo.toml",
    project_root / "canisters" / "root" / "Cargo.toml",
):
    if not candidate.is_file():
        continue

    with candidate.open("rb") as fh:
        cargo = tomllib.load(fh)

    workspace = cargo.get("workspace", {}).get("package", {})
    if "version" in workspace:
        print(workspace["version"])
        raise SystemExit(0)

    package = cargo.get("package", {})
    if "version" in package:
        print(package["version"])
        raise SystemExit(0)

raise SystemExit(
    "unable to determine template stage version; set CANIC_TEMPLATE_STAGE_VERSION explicitly"
)
PY
}

PROJECT_ROOT="${CANIC_PROJECT_ROOT:-$(discover_project_root)}"
CONFIG_PATH="$(discover_config_path)"
STAGE_VERSION="$(discover_stage_version)"
WASM_ROOT="${CANIC_STAGE_WASM_DIR:-${PROJECT_ROOT}/.dfx/${NETWORK}/canisters}"
TMP_STAGE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/canic-release-stage.XXXXXX")"

cleanup() {
    rm -rf "${TMP_STAGE_DIR}"
}

trap cleanup EXIT

if [ ! -d "${WASM_ROOT}" ] && [ "${NETWORK}" = "local" ] && [ -d "${PROJECT_ROOT}/.dfx/local/canisters" ]; then
    WASM_ROOT="${PROJECT_ROOT}/.dfx/local/canisters"
fi

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

generate_stage_inputs() {
    python3 - "${CONFIG_PATH}" "${WASM_ROOT}" "${STAGE_VERSION}" "${CHUNK_BYTES}" "${TMP_STAGE_DIR}" <<'PY'
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
        if role not in {"root", "wasm_store"}:
            roles.add(role)

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
}

call_root_method() {
    local method="$1"
    local arg_file="${2:-}"

    if [ -n "${arg_file}" ]; then
        dfx canister call "${ROOT_CANISTER}" "${method}" --network "${NETWORK}" --argument-file "${arg_file}" >/dev/null
    else
        dfx canister call "${ROOT_CANISTER}" "${method}" --network "${NETWORK}" >/dev/null
    fi
}

stage_role_dir() {
    local role_dir="$1"
    local role
    local template_id
    local artifact_path
    local payload_size_bytes
    local payload_size
    local chunk_count

    role="$(basename "${role_dir}")"
    template_id="embedded:${role}"
    artifact_path="${WASM_ROOT}/${role}/${role}.wasm.gz"
    payload_size_bytes="$(stat -c '%s' "${artifact_path}")"
    payload_size="$(format_bytes "${payload_size_bytes}")"
    chunk_count="$(find "${role_dir}" -name 'chunk-*.did' | wc -l | tr -d ' ')"

    echo "Staging role '${role}' as ${template_id}@${STAGE_VERSION} in root stable memory for later live wasm_store publication (${payload_size}, ${chunk_count} chunks)"

    call_root_method "canic_template_stage_manifest_admin" "${role_dir}/manifest.did"
    call_root_method "canic_template_prepare_admin" "${role_dir}/prepare.did"

    while IFS= read -r chunk_file; do
        call_root_method "canic_template_publish_chunk_admin" "${chunk_file}"
    done < <(find "${role_dir}" -name 'chunk-*.did' | sort)
}

wait_for_ready() {
    local attempt=0

    echo "Waiting for ${ROOT_CANISTER} to reach READY"
    for _ in $(seq 1 300); do
        if dfx canister call "${ROOT_CANISTER}" canic_ready --network "${NETWORK}" 2>/dev/null | grep -q "true"; then
            echo "Root bootstrap resumed and reached READY"
            return 0
        fi

        attempt=$((attempt + 1))
        if [ $((attempt % 10)) -eq 0 ]; then
            echo "Still waiting for ${ROOT_CANISTER} to reach READY; debug with:"
            echo "  dfx canister call ${ROOT_CANISTER} canic_wasm_store_bootstrap_debug --network ${NETWORK}"
            echo "  dfx canister call ${ROOT_CANISTER} canic_wasm_store_overview --network ${NETWORK}"
        fi
        sleep 1
    done

    echo "Root bootstrap resumed but did not reach READY within 300s" >&2
    echo "Bootstrap debug:" >&2
    dfx canister call "${ROOT_CANISTER}" canic_wasm_store_bootstrap_debug --network "${NETWORK}" || true
    echo "Wasm store overview:" >&2
    dfx canister call "${ROOT_CANISTER}" canic_wasm_store_overview --network "${NETWORK}" || true
    return 1
}

require_dfx
generate_stage_inputs

echo "Submitting staged release inputs to ${ROOT_CANISTER}"
echo "Project root: ${PROJECT_ROOT}"
echo "Config: ${CONFIG_PATH}"
echo "Artifacts: ${WASM_ROOT}"
echo "Template version: ${STAGE_VERSION}"
echo "Root embeds the bootstrap wasm_store module; only ordinary release roles are staged into root stable memory for later live wasm_store publication"

while IFS= read -r role_dir; do
    stage_role_dir "${role_dir}"
done < <(find "${TMP_STAGE_DIR}" -mindepth 1 -maxdepth 1 -type d | sort)

echo "Resuming root bootstrap after release staging so root can publish ordinary roles into the live wasm_store"
call_root_method "canic_wasm_store_bootstrap_resume_root_admin"
wait_for_ready
