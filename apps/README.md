# Apps

This directory contains config-defined Canic Apps. A directory belongs here
when it has a `canic.toml` that describes an App topology and should be
discoverable by `canic app list` and usable through commands that take the
App name as a positional argument.

The built-in `fleet_coordinator` and `wasm_store` roles are not sourced from
this directory. Their canonical canister crates live at
`crates/canic-fleet-coordinator/` and `crates/canic-wasm-store/` so downstreams
build the same infrastructure from published Canic sources instead of carrying
local built-in role crates. The local build helper discovers matching canonical
sources from the resolved `canic` package automatically. Each App still owns
its configuration-compiled Fleet Subnet Root under its own directory.

## Layout

- `test/` – local reference topology wired through `icp.yaml` and used by CI
  wasm/audit workflows.
  - `root/` – Fleet Subnet Root package (`canic::start!` with package metadata
    `app = "test"` and `role = "root"`) used to build the root infrastructure
    artifact and runtime endpoint bundle.
  - `app/` – minimal application canister used as a placeholder service.
  - `user_hub/` + `user_shard/` – sharding placement plus delegated signing flow.
  - `scale_hub/` + `scale/` – scaling pool demo, with the worker role exposed
    as `scale_replica`.
  - `canic.toml` – shared test topology referenced by each reference canister `build.rs`.
  - `test-configs/` – config fixtures used by local checks.
- `demo/` – small Component and sharding App for source/build experiments.
  - `root/` – Fleet Subnet Root package for the demo topology.
  - `app/` – simple Component role.
  - `user_hub/` + `user_shard/` – local sharding walkthrough roles with
    human-readable planning, assignment, and shard inspection endpoints.
  - `canic.toml` – shared demo topology referenced by each demo App canister `build.rs`.

## Local Workflow

The test Canisters are wired through `icp.yaml`; custom build steps invoke the
same host artifact builder used by `canic install`.

- Inspect the source topology: `canic app config test --verbose`
- Build one role: `canic build test app --profile fast`
- Install the managed test Fleet:
  `canic install test test-local --fleet-input <path> --profile fast`
- Create/build test canisters manually: `icp deploy -e test`

The installer provisions and activates the configured top-level `app`, `test`,
`user_hub`, and `scale_hub` Components before terminal Fleet publication.
`user_shard` and `scale_replica` descendants are created only by later
application/runtime requests. The demo sharding walkthrough is
`demo_user_hub_plan("alice")`,
`demo_user_hub_assign("alice")`, then
`demo_user_shard_describe("alice")` on the returned shard.

The separate Fleet input format is documented in
[`fleet-install-input.md`](../docs/architecture/fleet-install-input.md).
Isolated test probes and PocketIC fixtures live under `canisters/test/`.

Nonlocal targets expect their environment to be managed externally.
