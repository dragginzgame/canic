# Sample Canisters

The canisters in this directory exist purely for tests, demos, and examples.
Each crate exercises a different portion of the Canic stack so contributors can
try features end-to-end without touching production code.

## What’s Here

- `root/` – root orchestrator canister (`canic::start_root!`) that wires topology, imports child WASMs, and exposes root admin endpoints.
- `app/` – minimal “application” canister used as a placeholder service.
- `auth/` – signature/auth demo endpoints (`ops::ic::signature` helpers).
- `shard_hub/` + `shard/` – sharding pool demo (assign principals to shard workers).
- `scale_hub/` + `scale/` – scaling pool demo (spawn worker canisters under policy).
- `blank/` – minimal canister used for provisioning flows (create-canister requests).
- `test/` – timer and update/query coverage used by `make test-canisters`.

The shared demo topology lives in `crates/canisters/canic.toml` and is referenced by each canister’s `build.rs`.

## Local Workflow

These canisters are wired through `dfx.json` (custom build steps call `scripts/app/build.sh`).

- Start a clean local replica: `scripts/app/dfx_start.sh`
- Create/build canisters: `dfx canister create --all` then `dfx build --all`
- Run the scripted end-to-end flow: `make test-canisters` (or `make test`)

Note: `dfx build` uses `gzip=true`, so `.dfx/local/canisters/<name>/<name>.wasm.gz` is produced and embedded by the root canister as the “child WASM bundle”.
