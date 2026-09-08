# Apps

This directory contains config-defined Canic Apps. A directory belongs here
when it has a `canic.toml` that describes an App topology and should be
discoverable by `canic app list` and usable through commands that take the
App name as a positional argument.

Root, Coordinator and Store are Canic-owned infrastructure. The host generates
one thin Cargo package per role from the selected Canic dependency. Apps own
configuration and application canisters; they do not provide Fleet entrypoint
crates. Root generation selects the exact configured capabilities.

## Layout

- `test/` – local reference topology wired through `icp.yaml` and used by CI
  wasm/audit workflows.
  - `app/` – minimal application canister used as a placeholder service.
  - `index_hub/` + `index_child/` – indexed placement with children allocated
    on demand by the Hub.
  - `user_hub/` + `user_shard/` – sharding placement plus delegated signing flow.
  - `scale_hub/` + `scale/` – scaling pool demo, with the worker role exposed
    as `scale_replica`.
  - `test/` – standalone test role used by the reference topology.
  - `canic.toml` – shared test topology referenced by each reference canister `build.rs`.
  - `test-configs/` – config fixtures used by local checks.
- `demo/` – small Component and sharding App for source/build experiments.
  - `app/` – simple Component role.
  - `user_hub/` + `user_shard/` – local sharding walkthrough roles with
    human-readable planning, assignment, and shard inspection endpoints.
  - `canic.toml` – shared demo topology referenced by each demo App canister `build.rs`.
## Local Workflow

The test Canisters are wired through `icp.yaml`; custom build steps invoke the
same host artifact builder whose outputs are bound by `canic fleet ensure`.

- Inspect the source topology: `canic app config test --verbose`
- Build the complete App and Canic infrastructure artifact set:
  `canic build test`
- Build one role: `canic build test app`
- Build production-optimized artifacts explicitly:
  `canic build test --profile release`
- Review the managed test Fleet:
  `canic fleet ensure test-local --desired fleets/test-local.toml`
- Create/build test canisters manually: `icp deploy -e test`

The desired Fleet reconciler creates or reuses the configured top-level `app`,
`index_hub`, `scale_hub`, `test`, and `user_hub` canisters according to its
reviewed plan. `index_child`, `scale_replica`, and `user_shard` descendants are
created only by later application/runtime requests. The demo sharding
walkthrough is `demo_user_hub_plan("alice")`,
`demo_user_hub_assign("alice")`, then
`demo_user_shard_describe("alice")` on the returned shard.

The separate desired Fleet format is documented in
[Fleet ensure](../docs/features/operations/fleet-ensure.md).
Isolated test probes and PocketIC fixtures live under `canisters/test/`.

Nonlocal targets expect their environment to be managed externally.
