# Apps

This directory contains example Canic applications. Each App has a
`canic.toml` file describing its canister roles and layout. The `canic app list`
command discovers these directories, and other commands refer to an App by its
name.

Apps provide their own configuration and application canisters. Canic generates
the Root, Coordinator, and Store management canisters needed to deploy them;
application authors do not create those packages themselves.

## Layout

- `test/` – the larger reference App used by Canic's build and deployment tests.
  - `app/` – minimal application canister used as a placeholder service.
  - `index_hub/` + `index_child/` – indexed placement with children allocated
    on demand by the Hub.
  - `user_hub/` + `user_shard/` – sharding placement plus delegated signing flow.
  - `scale_hub/` + `scale/` – scaling pool demo, with the worker role exposed
    as `scale_replica`.
  - `test/` – standalone test role used by the reference topology.
  - `canic.toml` – shared test topology referenced by each reference canister `build.rs`.
  - `test-configs/` – config fixtures used by local checks.
- `demo/` – a smaller App for learning and local build experiments.
  - `app/` – simple Component role.
  - `user_hub/` + `user_shard/` – local sharding walkthrough roles with
    human-readable planning, assignment, and shard inspection endpoints.
  - `canic.toml` – shared demo topology referenced by each demo App canister `build.rs`.
## Local Workflow

The test canisters are also listed in `icp.yaml` for low-level local testing.
Normal Canic builds and `canic fleet ensure` use the same generated artifacts.

- Inspect the source topology: `canic app config test --verbose`
- Build the complete App and Canic infrastructure artifact set:
  `canic build test`
- Build one role: `canic build test app`
- Build production-optimized artifacts explicitly:
  `canic build test --profile release`
- After creating the desired Fleet document, review it:
  `canic fleet ensure test-local --desired fleets/test-local.toml`
- Create/build test canisters manually: `icp deploy -e test`

The test App exposes top-level Component Specs for `app`, `index_hub`,
`scale_hub`, `test`, and `user_hub`. The selected desired Fleet derives concrete
occurrences from explicit Component Group deployments; declaring a Spec alone
does not install it. `index_child`, `scale_replica`, and `user_shard`
descendants are created only by later application/runtime requests. The demo
sharding walkthrough is `demo_user_hub_plan("alice")`,
`demo_user_hub_assign("alice")`, then
`demo_user_shard_describe("alice")` on the returned shard.

The separate desired Fleet format is documented in
[Fleet ensure](../docs/features/operations/fleet-ensure.md).
Isolated test probes and PocketIC fixtures live under `canisters/test/`.

Nonlocal targets expect their environment to be managed externally.
