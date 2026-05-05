# Sample Canisters

The canisters in this directory are the reference demo topology.
They exercise the main Canic flows end to end without carrying internal
test-only helpers in the shipped demo surface.

The implicit `wasm_store` is no longer sourced from this directory. Its
canonical canister crate now lives at `crates/canic-wasm-store/` so downstreams
build the same store from published Canic sources instead of carrying a
local `wasm_store` crate. The local build helper discovers the matching
canonical store source from the resolved `canic` package automatically.

## What’s Here

- `root/` – root orchestrator canister (`canic::start_root!`) that wires topology, bootstraps the internal `wasm_store`, stages/publishes ordinary child releases, and exposes root admin endpoints.
- `app/` – minimal “application” canister used as a placeholder service.
- `user_hub/` + `user_shard/` – sharding placement plus delegated signing flow (hub does placement only; shard initiates delegation with root).
- `scale_hub/` + `scale/` – scaling pool demo (spawn replica canisters under policy).
- `minimal/` – minimal canister used as the shared runtime baseline canister.
- `playground/` – manual local sandbox for temporary endpoint experiments; it uses `canic::start_local!()` so `dfx deploy playground` does not require hand-written CANIC init args, and it is not part of the reference release set or test fixtures.

Internal correctness fixtures now live under `crates/canic-core/test-canisters/`, and internal audit probes now live under `crates/canic-core/audit-canisters/`. This keeps PocketIC and audit fixtures separate from the shipped demo topology.

The shared demo topology lives in `canisters/canic.toml` and is referenced by each canister’s `build.rs`.

## Local Workflow

These canisters are wired through `dfx.json` (custom build steps call `scripts/app/build.sh`, which is now a thin wrapper around the published `canic-build-canister-artifact` binary).

- Install the full local reference topology: `make demo-install`
- `root` stays thin: only the bootstrap `wasm_store` artifact is embedded, and the ordinary configured release set is staged after install from the build-produced `.dfx/local/canisters/root/root.release-set.json` manifest.
- Create/build canisters manually (dfx 0.30.2): `dfx canister create --all` then `dfx build --all`
- Run the scripted local smoke flow: `make test-canisters`

Note: `make demo-install` and `make test-canisters` now try one clean local
`dfx` restart automatically when `dfx ping local` fails. They are still manual
local smoke helpers, not part of `make test`, and nonlocal targets still
expect their replica to be managed externally.
