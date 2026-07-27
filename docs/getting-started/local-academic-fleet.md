# Local Academic Fleet Runbook

This runbook is for downstream projects that use a named local ICP CLI target
such as `academic` while developing a Canic-managed fleet. It focuses on the
integration traps that are easy to hit when Canic, raw `icp` commands, and shell
helpers are mixed in one workflow.

For the full managed-fleet shape, start with
[minimal-managed-fleet.md](minimal-managed-fleet.md). For general installation,
use [INSTALLING.md](../../INSTALLING.md).

## First Commands

Use Canic for Fleet-shaped operations. Before a Fleet reaches terminal catalog
publication, inspect source intent, replica status, and the install command's
journal-backed error:

```bash
canic status
canic app config <app> --verbose
canic --environment academic install <app> <fleet> --fleet-input <path>
```

For a terminal installed Fleet, add live inspection and Medic:

```bash
canic --environment academic info list <fleet>
canic --environment academic info env <fleet>
canic --environment academic medic fleet <fleet>
```

Use `canic app config <app>` to inspect what is configured and
`canic info list <fleet>` to inspect a terminal deployment. Treat the live
Fleet and Component Registry projections as the source for current Canister
IDs and the App config as the source for intended roles, metrics profiles, and
topology. Live inspection cannot reconstruct or bypass an incomplete install
journal.

## ICP Target Hygiene

Canic commands take a top-level `--environment <name>` for ICP-backed operations.
Before debugging target selection, confirm the shell resolves the expected ICP
CLI binary:

```bash
which icp
icp --version
```

`icp network update` updates the local network launcher used by the CLI; it
does not upgrade the `icp` binary itself. If Canic reports an unsupported ICP
CLI, use the upgrade command in [INSTALLING.md](../../INSTALLING.md#icp-cli-compatibility)
or pass top-level `--icp /path/to/icp` for a single Canic command.

Raw `icp` commands still need the ICP CLI target shape expected by your
project. In academic local scripts, prefer clearing stale shell target
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
canic --environment academic info env <fleet> > scripts/canister_ids.sh
source scripts/canister_ids.sh
```

For a terminal Fleet, `canic info env` reads live registered Canisters and
prints sourceable `CANIC_<ROLE>` exports such as `CANIC_ROOT`,
`CANIC_USER_HUB`, and `CANIC_USER_SHARD`. If a role appears more than once,
Canic prints numbered exports such as `CANIC_USER_SHARD_1` and
`CANIC_USER_SHARD_2`. Source the helper only after terminal installation and
after any reinstall that changes local Canister IDs.

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

## Fresh Install And Same-Release Recovery

Use `canic install <app> <fleet> --fleet-input <path>` for fresh local Fleet
creation or to recreate one after the ICP CLI replica lost state. The Fleet
input is separate operator-owned placement, admission, limit, and funding
policy; see
[`fleet-install-input.md`](../architecture/fleet-install-input.md). The Fleet
label and source App identity are independent. The local replica does not
persist canister state across stop/start.

The current 0.100 installer verifies the Coordinator, all planned roots, every
root-local Store, and every root's Registry `Joining` row, then deliberately
stops before snapshot synchronization, acknowledgement, activation, Component
creation, and terminal Fleet-catalog publication. Rerun the exact same install
command for same-release journal reconciliation; a conflicting Fleet input or
unresolved paid effect fails closed.

Every pre-1.0 release transition is reinstall-only. Do not use a raw
`icp canister install --mode=upgrade` command to carry a managed Fleet across
Canic releases, adopt an older installation, or bypass the current Registry
fence. Start the new release from empty Fleet state.

## Parent To Shard Calls

Parent-to-shard application calls use public delegated-token authenticated
endpoints. The presenting principal must match the signed token subject, and
the token grant for the target canister role must contain the endpoint's
required scope.

The current service-call contract is documented in
[ACCESS_ARCHITECTURE.md](../contracts/ACCESS_ARCHITECTURE.md#service-call-recipes).

```rust
#[canic::canic_update(
    name = "assign_project",
    requires(auth::authenticated("project.assign"))
)]
async fn assign_project(token: canic::dto::auth::DelegatedToken, request: AssignRequest)
    -> Result<MyResponse, canic::Error>
{
    // Endpoint auth verifies the reusable delegated token before handler code.
    Ok(assign_project_impl(token, request).await?)
}
```

Use public, non-internal application endpoints for raw external calls from
scripts or tests.

## Metrics And Deployed Wasm

`canic app config <app> --verbose` shows configured or inferred metrics
profiles. For a terminal Fleet, `canic info metrics <fleet> --kind <tier>`
queries what a deployed Canister actually exposes.

If a metrics tier reports `empty` or `canic_metrics` is unavailable, check all
three states before changing code:

```bash
canic app config <app> --verbose
canic --environment academic info list <fleet> --verbose
canic --environment academic info metrics <fleet> --kind core
```

The likely causes are: the role profile does not enable that tier, the deployed
Wasm predates the config change, or the canister was not rebuilt/reinstalled
after the change.

## Minimum Debug Loop

When a terminal Fleet looks wrong, run this loop before editing topology or
endpoint code:

```bash
canic status
canic app config <app> --verbose
canic --environment academic info list <fleet> --verbose
canic --environment academic info env <fleet>
canic --environment academic medic fleet <fleet>
canic --environment academic info metrics <fleet> --kind core --nonzero
```

This separates configured intent, deployed registry state, replica health,
readiness, module hashes, cycles, and runtime telemetry.
