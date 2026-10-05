# canic-cli

`canic-cli` provides the `canic` command-line program used by developers and
operators. It runs on your computer—not inside a canister—and handles App
setup, builds, local IC networks, deployment planning, diagnostics, backup, and
recovery.

Run `canic` without arguments, or run `canic --help`, to see the top-level help.
Use `canic <command> --help` for one command's options and examples.

The maintained command families are listed below. New users will most often
begin with `app`, `build`, `fleet`, `replica`, and `scaffold`.

```text
admission
app
auth
backup
build
component
cycles
diagnostic
evidence
fleet
frontend
info
inspect
medic
network
observatory
replica
restore
scaffold
state
status
token
toolchain
```

Authority-bearing Fleet commands read only a terminal `fleet ensure` inventory
and its exact Registry protocol bindings. This includes `info subnets`, which
requires a complete agreeing live
Coordinator Registry and Root-summary snapshot. `cycles funding` is protected
current status only.

Snapshot restoration uses `restore plan` for offline review, `restore prepare`
to validate artifacts and create or adopt the plan and journal, `restore run`
to preview or execute journaled operations, and `restore status` to inspect
progress. Backup references use the same row ordering as `backup list`.
`backup prune` counts verified copies for retention and protects unfinished
restores, including external journals. Its report includes skipped layouts and
partial deletion failures; see [local retention](../../docs/features/backup-and-restore/README.md#local-retention).

Fresh `backup create` execution is unavailable until Component Registry topology
preflight is implemented. It rejects before creating a layout or invoking ICP.
Dry-run planning does not create a backup or qualify
live authority. The accepted package-adapter work retains this limitation; see the
[backup availability boundary](../../docs/features/backup-and-restore/README.md#current-availability).

## Install

From a checkout:

```bash
cargo install --locked --path crates/canic-cli
canic help
```

From crates.io after publication:

```bash
cargo install --locked canic-cli --version <version>
```

Downstream workspaces should use the same `canic-cli` version as their `canic`
crate graph. The supported ICP CLI range is documented in the root
`INSTALLING.md`.

Canic artifact builds require the pinned `ic-wasm`; release builds additionally
require the checksum-authoritative Binaryen optimizer. The published installer
downloads, verifies, and prints both absolute executable paths without
requiring a Canic source checkout:

```bash
canic toolchain install
```

## App And Build

Create an App, declare or attach exact roles, and build deterministic Wasm:

```bash
canic app create <app>
canic app role attach <app> <role> --component-spec <component-spec>
canic build <app> <role> --provenance artifacts/<role>-provenance.json
```

Standalone builds default to the fast profile; select `--profile release` for
production artifacts. Canic keeps Wasm compilation non-incremental. Cargo owns
compiler-wrapper selection through `RUSTC_WRAPPER` or its wrapper configuration;
configure a compiler cache explicitly when wanted.

Complete builds report one verified cache hit or shared rebuild reason with the
artifact count. Add `--verbose` for tool/configuration details and the full
bounded cache explanation, including environment key names without values.
Checking and lock waiting use one live terminal line; redirected logs contain
plain phase events and separate check/lock timing.

## Fleet Ensure

`canic fleet ensure` owns desired-state installation and convergence.
`fleet bootstrap` prepares explicitly supplied infrastructure; `fleet import`
adds supplied capacity through an initialized Root. Their reviewed receipts
feed ordinary Ensure. See the [bootstrap](../../docs/features/operations/fleet-ensure.md#supplied-infrastructure-bootstrap)
and [capacity import](../../docs/features/operations/fleet-ensure.md#add-supplied-capacity-to-a-current-fleet)
procedures for those starting points.

For a retained estate, generate its low-level desired document from protected
Fleet policy, one finalized release build, and an explicit live-verified
estate seed:

```bash
canic fleet generate staging \
  --app-config apps/demo/canic.toml \
  --release-build <release-build-id>
```

The release set supplies artifacts and typed init contracts; it never invents
retained Principals. The seed supplies the exact live Fleet ID and exact
Coordinator, Root, Store, pool and treasury identities, which Canic verifies
through live management and protected Root inventory evidence. Fleet identity
does not derive from the environment name or operator. The treasury is one
explicitly adopted, already-present controlled canister; a missing identity
fails closed. This adoption path does not invent a treasury for a literally
empty estate. Every paid Root-owned pool asset must be seeded, including idle,
claimed and workload assets, so no controlled balance falls outside the
reviewed conservation equation. A workload remains the same conserved identity
without receiving idle-pool funding or being counted again by terminal
inventory. The generated contract binds the live Cycles Ledger fee and the seed's
explicit `management_creation_fee_cycles` for future capacity growth. Use `0B`
only when it is the exact applicable fee. A missing seeded canister is a blocker,
never a request to create a replacement.

For a literally empty estate, create or replay a durable no-effect seed before
generating the same desired-state contract:

```bash
canic fleet generate staging \
  --app-config apps/demo/canic.toml \
  --release-build <release-build-id> \
  --fresh \
  --management-creation-fee-cycles 500B
```

The fresh seed contains a random Fleet ID, exact Cycles Ledger and management
creation fee, and logical Coordinator, Root, Store and initial-pool roles. It
contains no invented Principal. Repetition accepts only the exact same seed
authority. Generation remains effect-free; the ordinary reviewed `fleet
ensure` plan/apply path creates each role with durable intent and resolves
dependent controllers and treasury authority from the retained creation
results. Use `--cycles-ledger <principal>` only for a network whose Cycles
Ledger differs from the maintained default.

Human cycle inputs require quoted TOML text or CLI values with exact
case-sensitive `B`, `T`, or `Q` suffixes, including decimals such as `1.5T`
and `0.1B`. Bare or unsuffixed quantities reject. Generated TOML always uses
at least `B`; durable plans retain exact integer cycle authority.

Because fresh Principals are outputs of the first reviewed plan, their typed
control-plane work may require a successor plan. If apply asks for a new plan
and retains the journal as `ReplanRequired`, run plan-only again and apply its
separately reviewed digest. Continue until the report is terminal; an immediate
post-terminal plan is effect-free.

The first invocation observes current state and retains a reviewed plan without
executing Fleet mutations:

```bash
canic fleet ensure staging --desired fleets/staging.toml
```

Review `plan_sha256`, all canister dispositions, the maximum operator debit,
fees, funding and burn, and the cycle-conservation equation. Then apply exactly
that plan:

```bash
canic fleet ensure staging \
  --desired fleets/staging.toml \
  --apply <plan_sha256>
```

Use `--json` for the complete stable report. Store publication chunks appear
as workspace-relative `.canic/fleet-ensure/objects/sha256/<digest>` paths with
their exact SHA-256 and byte size; raw payload bytes are never expanded into
the report. The current desired-state schema and retirement drain contract are
documented in [Fleet ensure](../../docs/features/operations/fleet-ensure.md).

The reconciler writes only current-generation state under:

```text
.canic/fleet-ensure/<environment>/<fleet>/
```

It does not scan or import former plan, deployment, recovery, or bundle paths.
An interrupted apply resumes the retained operation and reconciles the exact
effect before retry. A terminal immediate rerun produces no mutation actions.

## Cycle Safety

The reviewed plan separates:

- observed cycles in the controlled estate;
- cycles retained in reused canisters;
- cycles scheduled for treasury transfer;
- Cycles Ledger and management creation fees;
- bounded observation and update burn;
- requested new funding and maximum operator debit;
- create, reuse, reinstall, replace, and delete dispositions.

Apply stops if the selected account cannot cover the reviewed maximum, if the
plan or live authority drifts, or if actual debit/burn would exceed its bound.
A material canister cannot be replaced or deleted without an exact configured
treasury-bound drain endpoint. If the IC cannot recover those cycles safely,
Canic leaves the canister untouched and returns a typed blocker.
The update response alone is insufficient: stop and deletion remain fenced
until fresh observations prove both the bounded source debit and exact
treasury credit.

## Network, Replica, Evidence And State

Enroll exact network trust before connected operation. Obtain the expected
root-key SHA-256 fingerprint from the network operator through an authenticated
publication or independent trusted channel. Copy that value below and compare
the local digest with it. A fingerprint derived from the same untrusted key
download does not authenticate the network; Canic rejects a mismatch before
enrollment.

```bash
network_root_fingerprint='<64-lowercase-hex-from-the-network-operator>'
sha256sum ./root-key.der
canic network enroll local \
  --root-key ./root-key.der \
  --fingerprint "$network_root_fingerprint"
```

The `replica` group owns local launcher lifecycle. `evidence` validates and
gates stable evidence documents. `state` audits declared Canic metadata. Use
leaf `--help` output for the current grammar.

## Diagnostics

Look up one compact diagnostic by canonical code or decimal value:

```bash
canic diagnostic E123
canic diagnostic 123
```

Inspect a contended build without changing the lock or signalling its owner:

```bash
canic diagnostic build-lock --lock .canic/locks/complete-build-reuse.lock
canic diagnostic build-lock --lock .canic/locks/complete-build-reuse.lock --json
```

The report separates the kernel holder from advisory metadata, shows UTC phase
times and bounded process observations, and reports unavailable process visibility
explicitly. A quiet compiler or hidden PID does not establish a stalled or exited
owner. Cancel a redundant waiter if needed; never delete the lock file.

For argument-boundary debugging, `CANIC_TRACE_ARGV=1` prints every raw argument
before parsing. It may disclose secrets and should not be retained in shared
logs.

## Operator Components

Use `canic component plan <fleet> <name> --root <logical-root> --spec <spec>`
to retain one review. Apply its `review_sha256` with
`canic component apply <fleet> <name> --review <sha256>`. Root owns allocation,
installation and activation; the local name retains one operation across retries.
Each apply submits or resumes at most once, then polls status. Root coalesces
concurrent scheduling for the same operation. The default wait is 60 seconds; `--wait-secs 0` advances once. A pending result
is resumable using the same name and digest. `component status` refreshes progress
without submitting. `--json` emits the complete operation record and terminal
`progress.binding` with the Component identity, role and Principal.

`canic info env <fleet> --component-operation <name> --json` verifies that exact
operation's live completion before including its binding. It replaces a retained
Ready-pool row for the same Principal. It does not scan or silently adopt other
operations. A completed `component apply` replay returns its recorded result
without calls; use `component status` for a fresh live observation.

The review binds the terminal Fleet, installed release, Root/subnet, controller
set, selected identity and admitted Spec. Changes require deliberate resolution;
retain uncertain operation files. Files under `.canic/component-operations/` use
atomic writes, bounded reads and the same Fleet lock as Ensure. This command
claims existing Ready capacity and retains the installed Root funding policy;
it does not make a new funding transfer. `AUTO` in `app config` means automatic
role selection for a Spec. Initial instances require Component Group placement.

The [Component operation guide](../../docs/features/operations/component-operations.md)
describes review, exact retry, completion checks and environment export.

`frontend capacity`, `export` and `verify` provide the supported browser handoff
and external native-cycle preflight. See the
[frontend handoff guide](../../docs/features/operations/frontend-handoff.md).

The [Fleet observatory](../../docs/features/operations/fleet-observatory.md) supplies
independent role observations and bounded public reports from the host.

## Continue From Here

- [Install Canic](../../INSTALLING.md)
- [Operate a Fleet](../../docs/operations/README.md)
- [Browse all documentation](../../docs/README.md)
- [Back to the main README](../../README.md)
