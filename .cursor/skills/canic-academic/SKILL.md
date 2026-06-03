---
name: canic-academic
description: Use when working with Canic fleets, local academic ICP CLI targets, sharding, canic install or upgrade flows, canister ID shell helpers, canic info list, canic medic, canic metrics, or raw icp commands inside a Canic-managed downstream project.
---

# Canic Academic

Use this skill before changing a Canic-managed downstream fleet or debugging a
local `academic` deployment.

## First Checks

Run these before editing topology or endpoint code:

```bash
canic status
canic fleet config <fleet> --verbose
canic --network academic info list <deployment> --verbose
canic --network academic info env <deployment>
canic --network academic medic <deployment>
```

Use `fleet config` for configured intent and `info list` for deployed registry
truth. If they disagree, diagnose the deploy/install state before changing app
logic.

## ICP CLI Rules

- For Canic commands, pass the target explicitly: `canic --network academic ...`.
- For raw ICP CLI commands in this workflow, clear stale shell network state:
  `env -u ICP_NETWORK icp ... -e academic`.
- Do not rely on exported ambient network variables in scripts that also call
  Canic.
- Do not use `$ROOT` for a root canister principal. Use `CANIC_ROOT` and
  role-scoped names such as `CANIC_USER_HUB` and `CANIC_USER_SHARD`.
- Generate `scripts/canister_ids.sh` from deployed state with
  `mkdir -p scripts` and
  `canic --network academic info env <deployment> > scripts/canister_ids.sh`.
  Source it after install or reinstall and treat it as the canonical helper.

## Sourced Shell Helpers

Do not put `set -e` in scripts intended to be sourced into an interactive
shell. Use functions that return status instead. Executable scripts may still
use strict shell options.

## Install Or Upgrade

- `canic install <fleet>` is for fresh local creation or recreating local state
  after the ICP CLI replica lost canisters.
- If the canister exists and only Wasm changed, treat it as an upgrade flow.
  Check `canic info list` and `canic medic` before and after.
- If `canic install` is blocked on existing state, decide between reinstall,
  raw ICP upgrade, or deployment registration repair. Do not keep retrying the
  same install blindly.

## Sharded Calls

For protected Canic internal endpoints, call through generated endpoint
descriptors and pass the caller role explicitly:

```rust
use canic::__internal::core::api::ic::canic::CanicInternalClient;
use canic::ids::CanisterRole;

let response: MyResponse = CanicInternalClient::new(shard_pid)
    .call_update_result(
        &project_shard_endpoint(),
        CanisterRole::new("user_hub"),
        (tenant_id, request),
    )
    .await?;
```

Use tuple Candid arguments for multi-argument methods. Scripts and external
tests should call public, non-internal endpoints instead of protected internal
methods.

## Metrics

When `canic_metrics` is missing or a tier is empty, check:

```bash
canic fleet config <fleet> --verbose
canic --network academic info list <deployment> --verbose
canic --network academic metrics <deployment> --kind core --nonzero
```

Likely causes are a profile that does not enable the tier, stale deployed Wasm,
or a rebuild/reinstall that did not actually happen.
