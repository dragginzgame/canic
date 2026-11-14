# One autonomous profile worker

This experimental increment creates one usable profile worker after the operator
leaves. Normal growth and successful manager maintenance are now verified on
mainnet; injected interruption cases use synthetic cycles locally. The existing mainnet
entry/worker pair is unchanged. A separate manager owns the retained approval,
worker Wasm, timer and management-effect journal. The operator controls the
manager; the manager becomes the worker's sole controller.

There is one nonrenewable slot, no general workflow engine, Coordinator or
artifact-service canister. The existing entry has an immutable route and does
not discover or distribute users across additional workers. Use the new worker
directly with the profile client.

## Current mainnet demo

Deployed and verified on 2026-11-13:

| Role | Canister |
| --- | --- |
| Manager | `rk6n2-wyaaa-aaaad-qmoua-cai` |
| Autonomous worker | `rn7lo-3aaaa-aaaad-qmouq-cai` |

After admission, only anonymous observations ran on the host until the manager
reported `Ready`. It issued exactly one create, one upload and one install.
A subsequent certified update retained that evidence; independent certified
module/controller observations matched both approved artifacts. The manager
controls the worker, and `canic-mainnet` controls the manager.

On 2026-11-14 the manager was upgraded to the locally qualified maintenance
executable. Its full journal, worker principal and profile survived. Replaying
the original approval returned the same spent slot with counts `[1, 1, 1]`.
`growth-status` now reports `maintenance_candidate: true`; the marker identifies
the changed executable and does not grant new authority.

User 1 belongs to `canic-mainnet`, with revision 1 and display name
`Canic autonomous mainnet worker`. Use the existing deployment:

```sh
./target/debug/canic-profile --manager rk6n2-wyaaa-aaaad-qmoua-cai growth-status
./target/debug/canic-profile --canister rn7lo-3aaaa-aaaad-qmouq-cai get 1
canic fleet show --app profile --fleet mainnet --watch --manager rk6n2-wyaaa-aaaad-qmoua-cai \
  --wallet o5trf-oqyg7-cawjp-xs4pw-aomb3-iwki5-hyezf-qahfz-j3ffd-jh4fc-oqe \
  --link bisqm-biaaa-aaaas-qhfeq-cai:4pkhu-oaaaa-aaaai-ra2ta-cai
```

Open **http://localhost:8787/** manually to view the fleet; no browser launches
automatically. If its viewer
is already running, use that URL or choose another `--port`. Live checks begin
30 seconds after each completed check, and the browser reloads every 30 seconds.
Failed refreshes retain the prior snapshot with a visible warning. Ctrl-C in its
terminal stops both observation and serving.
For a one-shot terminal summary, omit `--watch` from the fleet command above.
Add `--json` when consuming the complete observation in a script.
It includes the four canisters in `profile/mainnet`: the original entry/worker,
the manager and the autonomous worker. Hello World belongs to the separate
`hello-world/mainnet` fleet. Manager queries and private profile reads
are explicitly uncertified; update replies and module/controller metadata use
their certified paths.
The optional wallet panel also shows the default ICP receive address and public
ICP/cycles-ledger balances. These exclude cycles already held by the canisters.
Use `canic wallet PRINCIPAL` for a fresh read or add `--address-only` for offline
address derivation; see the [wallet guide](../../README.md#check-the-mainnet-wallet).

Evidence is retained under `.canic/growth-mainnet/`, alongside the host deployment
journal for operation
`2cfe76adf294be8d03c2afeca2f5f64dc4be51ca4ba470dd66517f629dd6ce42`.
See [the mainnet record](../design/m0-first-journey.md#autonomous-mainnet-worker).
Maintenance evidence is in `.canic/growth-mainnet-maintenance/`; its
[preservation and receipt-recovery record](../design/m0-first-journey.md#mainnet-manager-maintenance)
includes the exact upgraded module and original operation identity.
The remaining fresh-deployment instructions create another manager and worker;
they are not needed to use these existing canisters. This manager's slot is spent.

## Run the local rehearsal

```sh
make profile
make growth-test
```

The existing prepared PocketIC server and ICP CLI are required. The test deploys
through Canic, generates and submits an approval through the native client, then
advances IC time without another mutating operator command. It checks one ready
worker, its controller, authenticated profile writes, manager maintenance and
no duplicate creation. The simulator closes when the test finishes.

The test retains `target/growth/journey-N/fleet.html`; use the directory from
your run and serve that snapshot in the normal browser:

```sh
canic fleet view target/growth/journey-N/fleet.html --port 8788 --open
```

Open **http://localhost:8788/** if the desktop opener is unavailable. Leave the
terminal running; Ctrl-C stops the viewer. This is a saved local observation,
not a live replica after the test ends.

## Prepare a fresh mainnet manager

These commands create a new deployment. Fund the selected cycles-ledger account
before applying; no funding conversion or debit occurs during a build or plan.
The proposed manager creation attaches 2.5T cycles. The resulting liquid balance
must cover the worker's 1T attachment plus at least 0.5T reserve. Creation,
execution and storage fees are separate from transferred cycles; this is not a
guarantee of indefinite operation or a quoted total cost.

```sh
mkdir -p target/growth
canic artifact build --json --package canic-profile-manager > target/growth/mainnet-manager.json
cp "$(jq -r .wasm target/growth/mainnet-manager.json)" target/growth/mainnet-manager.wasm
canic artifact build --json --package canic-profile-worker > target/growth/mainnet-worker.json
cp "$(jq -r .wasm target/growth/mainnet-worker.json)" target/growth/mainnet-worker.wasm
canic fleet plan --no-config --json --wasm target/growth/mainnet-manager.wasm \
  --network ic \
  --controller "$(./.tools/ic/bin/icp identity principal --identity canic-mainnet)" \
  --cycles 2500000000000 > target/growth/mainnet-plan.json
cat target/growth/mainnet-plan.json
canic fleet apply --no-config --json "$(jq -r .operation target/growth/mainnet-plan.json)" \
  --identity canic-mainnet --icp-bin ./.tools/ic/bin/icp \
  > target/growth/mainnet-receipt.json
```

Build without features for the initial live manager. `fault-injection` is for
local tests only. Retain `.canic/` and the receipt; the normal Canic host journal
owns signed deployment-attempt custody and recovery.

## Approve one worker and leave the host

```sh
manager=$(jq -r .canister target/growth/mainnet-receipt.json)
./target/debug/canic-profile --manager "$manager" growth-plan \
  --wasm target/growth/mainnet-worker.wasm \
  --output target/growth/mainnet-approval.json --delay-seconds 120
./target/debug/canic-profile --manager "$manager" growth-approve \
  target/growth/mainnet-approval.json
```

Review the plan output before approval: exact manager, artifact hash, fixed 1T
attachment and dispatch/deadline times. The retained file also binds the native
client to the exact selected network and optional local root key. Submit before
the scheduled time (within one hour of planning). Once accepted, the manager
owns the complete bytes and can proceed without the host. New effects must start
within ten minutes of the scheduled time. The slot cannot be replaced or renewed.

The client keeps a private copy of the approval and exact Wire request under
`.canic/growth-submissions/`. ICP CLI owns signing and submission. This command
does not use the host's signed deployment-attempt journal: safe repetition relies
on the manager's committed identical-approval check. After a lost admission reply,
inspect the manager; repeating the exact approval returns existing evidence and
cannot admit a second worker. The client never retries automatically.

```sh
./target/debug/canic-profile --manager "$manager" growth-status
./target/debug/canic-profile --manager "$manager" growth-reconcile
canic fleet show --no-config --manager "$manager" --port 8788 --open
```

`growth-status` and fleet manager reports are explicitly uncertified queries.
`growth-reconcile` is a controller-only update returning a certified reply; it
can make read-only management observations and schedule the next unissued phase.
It never repeats an issued create, upload or install. Fleet observation alone
does not resume work. Reported worker module/controller metadata is independently
checked using the fleet's certified observation path.

When the phase is `Ready`, copy its `worker` principal and use it directly:

```sh
./target/debug/canic-profile --canister WORKER put 1 --name "Autonomous worker"
./target/debug/canic-profile --canister WORKER get 1
```

## Failure and maintenance boundaries

Intent, attachment reservation and outgoing-call issuance share one IC message
commit. A returned result must survive the callback's successful completion;
writing the journal and then trapping does not retain it. Each next effect runs
in a separate timer message, so its failure cannot erase a prior committed result.

| Interruption | Retained outcome and maintenance behavior |
| --- | --- |
| Trap before creation dispatch commits | No child or committed attempt; original scheduled authority survives |
| Creation callback writes then traps | One child exists but its principal/refund is lost; `CreateIssued` remains blocked with full 1T attachment exposure |
| Later upload message traps before dispatch | Previously committed worker principal survives |
| Upload callback writes then traps | `UploadIssued`; stored-chunk observation can reconstruct its outcome |
| Install callback writes then traps | `InstallIssued`; module/controller observation can reconstruct its outcome |

The maintenance rehearsal upgrades the manager to a distinct
`maintenance-candidate` executable using the same storage format, then observes
and continues only unissued work. This successful transition has now also passed
on the existing mainnet manager. For a fresh rehearsal, build that feature and use the normal Canic
upgrade plan with the manager's exact principal, controller and previous module
hash. Never reinstall to resolve an uncertain slot: that destroys its authority
history. Creation-result loss has no automated recovery, even after maintenance.

If `canic fleet apply` stops after retaining an installation reply but before writing
its terminal receipt, inspect `canic fleet status OPERATION` and resume that same
operation. The retained reply owns settlement; preserve its files and original
digest. The mainnet maintenance run exercised this path and recovered its receipt
using certified reads with the signer unavailable. It did not issue another
installation. A retained reply alone is not a substitute for final module and
controller verification.

The operator must trust manager code: its controller can replace that code and
therefore ultimately controls the child. This slice exposes no worker-upgrade
or controller-transfer command. Supporting those operations requires a future
manager upgrade and separately qualified authority; do not treat this as a
general production fleet manager.

Bounds: one approval; at most one create, upload and install dispatch; raw worker
Wasm at most 512 KiB; 4 KiB status frames; two IC Storage cells; at most 64 stable
pages (4 MiB). IC Timers owns one local registration. There is no recurring idle
poll or reset endpoint. Expiry or insufficient reserve stops new effects without
freeing the original slot. The management chunk store additionally retains one
copy of the approved worker module (at most 512 KiB); it is not included in the
stable-page count and has no cleanup operation in this slice. No cross-schema
migration is supplied.
