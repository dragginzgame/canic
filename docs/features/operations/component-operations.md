# Operator Component operations

`canic component` provisions one ordinary top-level Component on a selected
active Root. Fleet Ensure owns initial Component Group deployment; Root owns
allocation, installation, activation and Directory synchronization.

## Review and apply

Use a terminal current-release Fleet and an ICP identity that controls the
selected Root. Select its logical name from the reviewed Fleet desired state
and an admitted Component Spec from the App configuration:

```sh
canic --environment local component plan demo extra --root root --spec core --json > component-review.json
review=$(jq -er '.plan.review_sha256' component-review.json)
canic --environment local component apply demo extra --review "$review" --json
```

`plan` observes authority and available Ready capacity, then retains a local
review. It sends no provisioning command. The review binds the selected
network, Fleet, Root/subnet, release, module/Candid, controller, Spec and role.
`AUTO` selects build roles; it does not create an initial Component placement.

Applying reserves one submission before sending Root's existing command.
The command polls for up to 60 seconds by default. `--wait-secs 0` advances
once; the maximum selectable polling window is 3,600 seconds. A successful
incomplete response is pending work, not a ready Component. Check
`progress.complete` in JSON before using its Principal.

## Interruption and export

Repeat `component apply` with the same Fleet, local operation name and review
digest after an interrupted command. The original operation ID remains bound
to its selected release. An incomplete existing operation resumes Root's driver
after same-release restoration; it does not allocate a second identity.
Changing authority fails instead of retargeting an issued operation.

An unchanged-Fleet Ensure review may have a new plan hash. That hash records
the Component review's original provenance; it does not strand the operation
when the live network, registry, Root, release, controller and Spec bindings
still match. Retry, status and export retain the original review and operation
identities. Actual authority changes still reject.

Subsequent Fleet Ensure inventory includes ordinary Components alongside initial
Component Groups. Each ordinary Component must join an exact Root pool claim,
active registry partition and completed allocation, with the current release,
module and controllers. Root admission and descendant limits still apply.

`component status demo extra --json` refreshes observations without submitting.
A completed apply returns its retained receipt without network calls. Use status
when a fresh live observation is required. Typed Root failures are preserved;
review and repair capacity before retrying a rejected request.

```sh
canic --environment local info env demo --component-operation extra --json > frontend-environment.json
```

This explicitly refreshes the selected operation and exports its terminal role
and Principal. If the Principal was previously a Ready pool asset, its exported
role is replaced. Fleet Ensure's retained inventory is not rewritten. The output
contains canister identifiers, not controller or provisioning credentials.
If current Fleet inventory changes during the live Component observation,
export rejects so one output cannot combine different Fleet observations.

Operations are stored under `.canic/component-operations/<environment>/<fleet>/`.
Retain that directory with the same-release Fleet workspace during recovery.
These commands do not provide cross-release adoption or migration.

## Qualification

The combined [local Fleet case](local-development-fleet.md#qualification) starts
from fresh Ensure convergence and qualifies a changed no-op review, lost reply,
ordinary Component inventory, single-submission retry and exact environment
export. It also checks explicitly imported Root-owned capacity and restart.

Focused native cases cover authority, corruption, intent persistence, monotonic
progress and replay. A real Root test covers lost response, same-release
restoration, duplicate resume, denied caller and effect-free terminal replay.
A second disposable Fleet exercises the public CLI through an isolated native
ICP identity, exact Candid sidecars, lost response, completion and JSON export.
Its starting terminal inventory is an explicit fixture; it is not a new complete
Fleet Ensure proof or an immutable published-package adoption result. Its `ic`
identity is synthetic and routed only to its isolated PocketIC gateway.
