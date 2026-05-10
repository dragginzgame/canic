# Fleets

This directory contains config-defined Canic fleets. A directory belongs here
when it has a `canic.toml` that describes a fleet topology and should be
discoverable by `canic fleet list` and usable through commands that take the
fleet name as a positional argument.

The implicit `wasm_store` is not sourced from this directory. Its canonical
canister crate lives at `crates/canic-wasm-store/` so downstreams build the same
store from published Canic sources instead of carrying a local `wasm_store`
crate. The local build helper discovers the matching canonical store source
from the resolved `canic` package automatically.

## Layout

- `test/` – local reference topology wired through `icp.yaml` and used by CI wasm/audit workflows.
  - `root/` – root orchestrator canister (`canic::start_root!`) that wires topology, bootstraps the internal `wasm_store`, stages/publishes ordinary child releases, and exposes root admin endpoints.
  - `app/` – minimal application canister used as a placeholder service.
  - `user_hub/` + `user_shard/` – sharding placement plus delegated signing flow.
  - `scale_hub/` + `scale/` – scaling pool demo.
  - `canic.toml` – shared test topology referenced by each reference canister `build.rs`.
  - `test-configs/` – config fixtures used by local checks.
- `demo/` – minimal root-plus-app fleet for quick experiments.
  - `root/` – root canister for the demo topology.
  - `app/` – simple application canister auto-created by the root.
  - `canic.toml` – shared demo topology referenced by each demo fleet canister `build.rs`.

## Local Workflow

The test canisters are wired through `icp.yaml`; custom build steps call
`scripts/app/build.sh`, which is a thin wrapper around `canic build`.

- Install the full local reference topology: `make test-fleet-install`
- `root` stays thin: only the bootstrap `wasm_store` artifact is embedded, and the ordinary configured release set is staged after install from `.icp/local/canisters/root/root.release-set.json`.
- Create/build test canisters manually: `icp deploy -e test`
- Run the scripted local smoke flow: `make test-canisters`

The demo fleet is intentionally small. Isolated test probes and PocketIC
fixtures live under `canisters/test/`.

Note: `make test-fleet-install` and `make test-canisters` are manual local smoke
helpers, not part of `make test`, and nonlocal targets expect their environment
to be managed externally.
