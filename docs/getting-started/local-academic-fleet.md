# Local Academic Fleet Runbook

This runbook is for downstream projects that use a named local ICP CLI target
such as `academic` while developing a Canic-managed fleet. It focuses on the
integration traps that are easy to hit when Canic, raw `icp` commands, and shell
helpers are mixed in one workflow.

For the full managed-fleet shape, start with
[minimal-managed-fleet.md](minimal-managed-fleet.md). For general installation,
use [INSTALLING.md](../../INSTALLING.md).

## First Commands

Use Canic for fleet-shaped operations and start every debugging pass with the
installed registry and medic checks:

```bash
canic status
canic --network academic info list <deployment>
canic --network academic info env <deployment>
canic --network academic medic <deployment>
```

Use `canic fleet config <fleet>` to inspect what is configured and
`canic info list <deployment>` to inspect what is deployed. If those disagree,
treat the deployed root registry as the source for current canister IDs and
the fleet config as the source for intended roles, metrics profiles, and
topology.

## ICP Target Hygiene

Canic commands take a top-level `--network <name>` for networked operations.
Raw `icp` commands still need the ICP CLI target shape expected by your
project. In academic local scripts, prefer clearing stale shell network
selection before passing the explicit ICP environment:

```bash
env -u ICP_NETWORK icp canister status <canister> -e academic
env -u ICP_NETWORK icp canister call <canister> <method> '(<args>)' -e academic
```

Do not mix an exported `ICP_NETWORK` with an explicit `-e academic` in the same
wrapper. If a helper calls both Canic and raw `icp`, pass the target explicitly
to each command instead of relying on ambient shell state.

## Canister ID Variables

Avoid using `ROOT` for a root canister principal in scripts. `ROOT` is commonly
read as a repository or filesystem root by humans and agents.

Use role-scoped names:

```bash
mkdir -p scripts
canic --network academic info env <deployment> > scripts/canister_ids.sh
source scripts/canister_ids.sh
```

`canic info env` reads the installed root registry and prints sourceable
`CANIC_<ROLE>` exports such as `CANIC_ROOT`, `CANIC_USER_HUB`, and
`CANIC_USER_SHARD`. If a role appears more than once, Canic prints numbered
exports such as `CANIC_USER_SHARD_1` and `CANIC_USER_SHARD_2`. Source the
helper after installation and after any reinstall that changes local canister
IDs.

## Sourced Helpers

Do not put `set -e` in helper scripts that developers source into an
interactive shell. A failed `icp` call can otherwise make the shell feel broken
or exit the session.

Use functions that return status instead:

```bash
canic_academic_status() {
  env -u ICP_NETWORK icp canister status "$1" -e academic
}
```

Executable scripts may still use strict shell options. Keep sourced helpers
boring and explicit.

## Install Versus Upgrade

Use `canic install <fleet>` for fresh local fleet creation or to recreate a
local deployment after the ICP CLI replica lost state. The local replica does
not persist canister state across stop/start.

When a canister already exists and you only need new Wasm on that canister,
treat it as an upgrade flow. Until a dedicated Canic upgrade wrapper is
available for that path, record the raw ICP command in the project runbook and
run `canic info list` plus `canic medic` before and after the upgrade.

```bash
canic --network academic info list <deployment>
canic --network academic medic <deployment>
env -u ICP_NETWORK icp canister install <canister> --mode=upgrade --wasm <path> -e academic
canic --network academic info list <deployment>
```

If `canic install` is blocked on an existing local deployment, do not keep
retrying the same install. Decide whether the project needs a fresh reinstall,
a raw ICP upgrade, or a deployment registration fix.

## Parent To Shard Calls

For Canic-protected internal endpoints, the hub or parent canister should call
the shard through the generated protected endpoint descriptor and an explicit
caller role. Do not hand-build the internal envelope in application code. The
canonical recipes live in
[ACCESS_ARCHITECTURE.md](../contracts/ACCESS_ARCHITECTURE.md#protected-internal-call-recipes).

```rust
use canic::__internal::core::api::ic::canic::CanicInternalClient;
use canic::ids::CanisterRole;

let result: MyResponse = CanicInternalClient::new(shard_pid)
    .call_update_result(
        &project_shard_endpoint(),
        CanisterRole::new("user_hub"),
        (tenant_id, request),
    )
    .await?;
```

Use tuple Candid arguments for multi-argument methods. Use public, non-internal
application endpoints for raw external calls from scripts or tests.

## Metrics And Deployed Wasm

`canic fleet config <fleet> --verbose` shows configured or inferred metrics
profiles. `canic metrics <deployment> --kind <tier>` queries what the deployed
canister actually exposes.

If a metrics tier reports `empty` or `canic_metrics` is unavailable, check all
three states before changing code:

```bash
canic fleet config <fleet> --verbose
canic --network academic info list <deployment> --verbose
canic --network academic metrics <deployment> --kind core
```

The likely causes are: the role profile does not enable that tier, the deployed
Wasm predates the config change, or the canister was not rebuilt/reinstalled
after the change.

## Minimum Debug Loop

When something looks wrong, run this loop before editing topology or endpoint
code:

```bash
canic status
canic fleet config <fleet> --verbose
canic --network academic info list <deployment> --verbose
canic --network academic info env <deployment>
canic --network academic medic <deployment>
canic --network academic metrics <deployment> --kind core --nonzero
```

This separates configured intent, deployed registry state, replica health,
readiness, module hashes, cycles, and runtime telemetry.
