# Fleets

This directory contains config-defined Canic fleets. A directory belongs here
when it has a `canic.toml` that describes a fleet topology and should be
discoverable by `canic fleet list`, `canic fleet use`, and implicit
`canic install` config selection.

The implicit `wasm_store` is not sourced from this directory. Its canonical
canister crate lives at `crates/canic-wasm-store/` so downstreams build the same
store from published Canic sources instead of carrying a local `wasm_store`
crate. The local build helper discovers the matching canonical store source
from the resolved `canic` package automatically.

## Layout

- `demo/` – local reference topology wired through `dfx.json`.
  - `root/` – root orchestrator canister (`canic::start_root!`) that wires topology, bootstraps the internal `wasm_store`, stages/publishes ordinary child releases, and exposes root admin endpoints.
  - `app/` – minimal application canister used as a placeholder service.
  - `user_hub/` + `user_shard/` – sharding placement plus delegated signing flow.
  - `scale_hub/` + `scale/` – scaling pool demo.
  - `minimal/` – shared runtime baseline canister.
  - `canic.toml` – shared demo topology referenced by each demo canister `build.rs`.
  - `test-configs/` – config fixtures used by local/demo checks.
- `test/` – internal correctness and PocketIC canister fixtures. `runtime_probe/` replaces the older `canister_test` name.

## Local Workflow

The demo canisters are wired through `dfx.json`; custom build steps call
`scripts/app/build.sh`, which is a thin wrapper around `canic build`.

- Install the full local reference topology: `make demo-install`
- `root` stays thin: only the bootstrap `wasm_store` artifact is embedded, and the ordinary configured release set is staged after install from `.dfx/local/canisters/root/root.release-set.json`.
- Create/build demo canisters manually: `dfx canister create --all` then `dfx build --all`
- Run the scripted local smoke flow: `make test-canisters`

Note: `make demo-install` and `make test-canisters` try one clean local `dfx`
restart automatically when `dfx ping local` fails. They are manual local smoke
helpers, not part of `make test`, and nonlocal targets expect their replica to
be managed externally.
