# canic-cli

`canic-cli` publishes the `canic` operator binary. It is the command-line
surface for fleet setup, role lifecycle inspection, artifact builds, stable
evidence output, passive deployment catalog inspection, policy gates, local
fleet installs, snapshots, backup validation, and guarded restores.

The compact v1 operator path is deliberately explicit: create or select a
fleet, scaffold and attach roles, build attached roles with provenance, check
saved deployment evidence, gate that evidence with policy, and inspect known
deployment-target state. Backup and restore commands remain available for
snapshot workflows; where they perform live snapshot or restore mutations,
Canic owns topology selection, manifests, journals, readiness checks, restore
ordering, and runner state around the underlying `icp` calls.

`canic-cli` intentionally keeps a narrow Rust library surface: external callers
should treat the installed `canic` binary as the operator interface. Host-side
build/install/fleet helpers live in `canic-host`, and backup/restore contracts
live in `canic-backup`.

## Install

Install from a checkout:

```bash
cargo install --locked --path crates/canic-cli
canic help
```

Install from crates.io after a release:

```bash
cargo install --locked canic-cli --version <version>
```

Downstream projects should install the same `canic-cli` version as their
`canic` crate dependency.

## Compact V1 Operator Surface

The maintained v1 command set keeps setup, build, evidence, policy, and local
catalog inspection separate:

```bash
canic fleet create <fleet>
canic scaffold canister <fleet> <role>
canic fleet role attach <fleet> <role> --subnet <subnet>
canic build <fleet> <role> --provenance artifacts/<role>-provenance.json
canic deploy check <deployment> --format envelope-json
canic evidence gate --policy policy.toml --envelope evidence.json
canic evidence gate --policy policy.toml --manifest evidence-manifest.json
canic deploy inspect catalog list
canic deploy inspect catalog inspect <deployment>
canic nns subnet list
canic nns node list
```

These commands do not imply one-command deployment, controller mutation,
artifact registry import, teardown, deployment groups, or signing. The only
topology mutation in that list is the explicit `fleet role attach` command.

The installed binary also includes the artifact builder:

```bash
canic build <fleet> <role>
```

To archive CI-friendly build provenance next to an artifact, request an
explicit provenance file:

```bash
canic build <fleet> <role> --provenance artifacts/<role>-provenance.json
```

For downstream repos where the Cargo workspace and ICP project root differ,
pass paths as command options instead of exporting Canic build environment
variables:

```bash
canic build \
  --profile fast \
  --workspace <cargo-workspace-dir> \
  --icp-root <icp-project-dir> \
  --config fleets/<fleet>/canic.toml \
  <fleet> \
  root
```

For a full local development setup, including ICP CLI, helper tools, and the
`canic` CLI, use the root `INSTALLING.md` guide.

## NNS Inspection

Public IC registry metadata is inspected through the `nns` namespace:

```bash
canic nns subnet refresh
canic nns registry version
canic nns subnet list
canic nns subnet list --verbose
canic nns subnet info <subnet|canister|subnet-prefix|deployment-target>
canic nns data-center list
canic nns data-center list --verbose
canic nns data-center info <data-center-prefix>
canic nns data-center refresh
canic nns node list
canic nns node list --verbose
canic nns node info <node-prefix>
canic nns node refresh
canic nns node list --data-center <data-center-prefix>
canic nns node list --node-provider <node-provider-prefix>
canic nns node list --node-operator <node-operator-prefix>
canic nns node list --subnet <subnet-prefix> --kind system
canic nns node-provider list
canic nns node-provider list --verbose
canic nns node-provider info <node-provider-prefix>
canic nns node-provider refresh
canic nns node-operator list
canic nns node-operator list --verbose
canic nns node-operator info <node-operator-prefix>
canic nns node-operator refresh
canic nns topology refresh
canic nns topology summary
canic nns topology coverage
canic nns topology versions
canic nns topology health
canic nns topology gaps
```

Cached `list` and `info` commands read mainnet NNS-derived reports. Use
`refresh` to force a new NNS registry read. The command group defaults to
`--network ic` and rejects local or custom networks.

## Local Install And Registry Commands

For local installed-fleet workflows, the CLI also exposes install, registry,
replica, backup, and restore commands.

Show local test-fleet canisters that already have ids:

```bash
canic --network local info list test
canic --network local info env test
```

`canic info list <name>` reads the installed root registry for that fleet.
Use `--subtree <name-or-principal>` to print one subtree with that node as the
rendered root.
`canic info env <name>` prints sourceable `CANIC_<ROLE>` canister ID exports
for scripts and local shell helpers.
Live list sources call `canic_ready` for each listed canister and include a
`READY` column with `yes`, `no`, or `error`, plus a `CYCLES` balance column.

If the list only shows the `root` row, the project has reserved a local root id
but has not installed the tree. Run `canic install test`, then use
`canic --network local info list test` to read the installed root registry.

Install and bootstrap the local fleet:

```bash
canic install test
```

`canic install <fleet>` uses `fleets/<fleet>/canic.toml`, the conventional
`root` ICP canister name, and Canic's built-in readiness timeout:

```bash
canic install test
```

The selected install config must include a fleet identity:

```toml
[fleet]
name = "test"
```

Successful installs write
`.canic/<network>/deployments/<deployment>.json` with the deployment name, fleet
template, root target, resolved root principal, build target, config path, root
verification state, and staging manifest path. `canic fleet config <name>` shows the
selected fleet declaration, including opt-in role features such as auth,
sharding, and scaling, while `canic info list <name>` queries the deployed root
registry for that target.
Commands use network `local` unless you pass
`--network <name>`.

The local ICP CLI replica does not persist canister state across stop/start.
If `canic status` reports a local fleet as `lost`, reinstall the fleet before
running backup or restore commands against that local environment. `canic status`
and `canic replica status` show the configured local gateway port; use
`canic replica start --port <port>` to require this project's `icp.yaml`
`gateway.port` to match before starting. Use `canic replica status --json` when
scripts need the structured ICP CLI local-network status payload.
Fleet configs live under the project-root `fleets/` directory. Commands
launched from nested directories discover that outer project root and keep
ICP project config plus `.icp/` and `.canic/` state there.

List saved fleet configs:

```bash
canic fleet list
canic fleet delete demo
canic --network ic fleet list
```

Create a new root-plus-app fleet:

```bash
canic fleet create my_app --yes
canic install my_app
```

Diagnose the named fleet, replica reachability, saved config path, and root
readiness:

```bash
canic info medic test
```

For downstream projects that combine Canic commands with raw `icp` calls on a
named local target such as `academic`, use the
[local academic fleet runbook](../../docs/getting-started/local-academic-fleet.md).
It covers target selection, canister ID helper naming, sourced shell helpers,
sharded calls, metrics checks, and install versus upgrade decisions.

Run command-specific help when you need exact flags:

```bash
canic <command> help
```

The installed CLI version is visible in top-level help and from `canic
--version`. The version flag is accepted at any command depth, so `canic backup
verify --version` reports the binary version instead of running the command.

## Backup Happy Path

Create a topology-aware backup:

```bash
canic backup create test
canic backup list
```

Non-dry-run captures recompute the selected topology immediately before
snapshot creation and fail if the topology hash changed since discovery. This
keeps subtree backups from silently crossing a registry change.

ICP CLI creates snapshots only for stopped canisters. Canic stops selected
members, creates snapshots, restarts them, downloads artifacts, verifies
checksums, and writes manifest/journal state under the backup directory.

Verify the captured backup directory:

```bash
canic backup verify 1
```

Verification is no-mutation. It validates the manifest, journal agreement,
durable artifact paths, and checksums before restore planning.

## Backup Checks

Use these commands after capture and before restore planning:

- `canic backup manifest validate` checks manifest shape, topology hash inputs,
  and backup units.
- `canic backup status` summarizes resumable download journal progress.
- `canic backup verify` validates the backup layout and artifact checksums.

For the normal operator restore path, prepare the selected backup row once,
inspect the prepared journal, then advance it with the guarded runner:

```bash
canic restore prepare 1 --require-verified --require-restore-ready
canic restore status 1 --require-no-attention
canic restore run 1 --dry-run
canic restore run 1 --execute --max-steps 1 --require-no-attention
canic restore status 1 --require-complete --require-no-attention
```

`restore prepare` writes `restore-plan.json` and
`restore-apply-journal.json` inside the selected backup directory, so later
restore commands can use the same backup row number from `canic backup list`.
For deeper no-mutation checks, use `canic restore plan`,
`canic restore apply --dry-run`, and `canic restore run --dry-run` directly.

## Restore Planning

Restore starts from a manifest, not from loose snapshot files:

```bash
canic restore plan \
  1 \
  --mapping restore-map.json \
  --out restore-plan.json \
  --require-verified \
  --require-restore-ready
```

Planning performs no mutations. It validates mapping, identity mode, snapshot
provenance, verification coverage, artifact checksums when requested, and
restore ordering. Plans include operation counts and parent-before-child
ordering metadata so operators can see the intended restore sequence before any
target is touched.

Render operations and create an apply journal:

```bash
canic restore prepare \
  1 \
  --mapping restore-map.json \
  --require-verified \
  --require-restore-ready
```

`restore prepare` is a convenience wrapper around verified planning and apply
journal creation. `restore apply` still exists for explicit plan-file
workflows and currently requires `--dry-run`; direct mutation through that
command is intentionally disabled. The generated journal is the input to the
guarded runner.

## Guarded Runner

Preview the maintained runner path without calling `icp`:

```bash
canic --network local restore run \
  1 \
  --dry-run \
  --out restore-run-dry-run.json
```

Execute a cautious one-step batch:

```bash
canic --network local restore run \
  1 \
  --execute \
  --max-steps 1 \
  --out restore-run.json \
  --require-no-attention
```

The native runner checks journal readiness, claims the next operation, runs the
generated `icp` command, marks the operation completed or failed, and persists
the journal after each transition. A normal ready journal includes snapshot
upload, canister stop, snapshot load, canister start, and verification
operations. `--max-steps 1` is the safest operational mode while validating a
new restore path. Snapshot load operations first run `icp canister status` and
fail before loading unless the target is visibly stopped.

If a previous runner stopped after claiming work, release the pending operation
back to ready:

```bash
canic restore run \
  1 \
  --unclaim-pending \
  --out restore-run-recovery.json
```

If an operation failed and you have inspected the failure, move it back to
ready before rerunning execution:

```bash
canic restore run \
  1 \
  --retry-failed \
  --out restore-run-retry.json
```

## Restore Journal Tools

Use `canic restore run --dry-run` to inspect the journal produced by
`restore apply --dry-run`. The runner preview includes progress, blocked work,
pending claims, failed operations, completion counts, and the next command
preview.

`canic restore run` is also the only maintained command for advancing a restore
journal. It owns command preview, claiming, execution, completion/failure
records, and pending-operation recovery.

## Safety Model

- Directory data may select a root, but topology defines membership.
- Captures fail closed when the selected topology hash changes before snapshot
  creation.
- Backup manifests carry topology, unit, identity, snapshot, artifact,
  provenance, and verification metadata.
- Restore planning is no-mutation and must prove mapping, ordering, checksum,
  verification, and snapshot-restore readiness before execution.
- Runner summaries and journals are durable audit artifacts; failures still
  write status before returning a nonzero exit code.
