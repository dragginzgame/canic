# Sample Canisters

The canisters in this directory exist purely for tests, demos, and examples.
Each crate exercises a different portion of the Canic stack so contributors can
try features end-to-end without touching production code.

## What’s Here

- `root/` – root orchestrator canister (`canic::start_root!`) that wires topology, bootstraps the internal `wasm_store`, stages/publishes ordinary child releases, and exposes root admin endpoints.
- `app/` – minimal “application” canister used as a placeholder service.
- `user_hub/` + `user_shard/` – sharding placement plus delegated signing flow (hub does placement only; shard initiates delegation with root).
- `scale_hub/` + `scale/` – scaling pool demo (spawn replica canisters under policy).
- `minimal/` – minimal canister used as the shared runtime baseline canister.
- `test/` – timer and update/query coverage used by `make test-canisters`.

The shared demo topology lives in `canisters/canic.toml` and is referenced by each canister’s `build.rs`.

## Local Workflow

These canisters are wired through `dfx.json` (custom build steps call `scripts/app/build.sh`).

- Start a clean local replica in another terminal: `scripts/app/dfx_start.sh`
- Install the full local reference topology: `make demo-install`
- The normal `root` install path no longer needs a separate release-staging helper; `dfx build --all` plus `dfx canister install root` is sufficient when the child role artifacts are present.
- Create/build canisters manually (dfx 0.30.2): `dfx canister create --all` then `dfx build --all`
- Run the scripted end-to-end flow: `make test-canisters` (or `make test`)

Note: `make demo-install` and `make test-canisters` assume `dfx` is already
running. They fail fast if the local replica is not available.
