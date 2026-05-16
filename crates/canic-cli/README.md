# canic-cli

`canic-cli` publishes the `canic` operator binary. It is the command-line
surface for installing local Canic fleets, selecting fleet configs, capturing
canister snapshots, validating backup artifacts, and preparing guarded
restores.

The CLI wraps ICP CLI for live snapshot and restore mutations. Canic
owns the topology selection, manifests, journals, readiness checks, restore
ordering, and runner state around those `icp` calls.

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
`canic` crate dependency. The installed binary includes the artifact builder:

```bash
canic build <role>
```

For downstream repos where the Cargo workspace and ICP project root differ,
pass paths as command options instead of exporting Canic build environment
variables:

```bash
canic build --profile fast --workspace backend --icp-root . --config fleets/toko/canic.toml root
```

For a full local development setup, including ICP CLI, helper tools, and the
`canic` CLI, use the install script in the root README.

## First Commands

Show local test-fleet canisters that already have ids:

```bash
canic --network local info list test
```

`canic info list <name>` reads the installed root registry for that fleet.
Use `--subtree <name-or-principal>` to print one subtree with that node as the
rendered root.
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

Successful installs write `.canic/<network>/fleets/<fleet>.json` with the root
target, resolved root principal, build target, config path, and staging
manifest path. `canic config <name>` shows the selected fleet declaration,
including opt-in role features such as auth, sharding, and scaling,
while `canic info list <name>` queries the deployed root registry for that
fleet.
Commands use network `local` unless you pass
`--network <name>`.

The local ICP CLI replica does not persist canister state across stop/start.
If `canic status` reports a local fleet as `lost`, reinstall the fleet before
running backup or restore commands against that local environment. `canic status`
and `canic replica status` show the configured local gateway port; use
`canic replica start --port <port>` to update this project's `icp.yaml`
`gateway.port` before starting. Use `canic replica status --json` when scripts
need the structured ICP CLI local-network status payload.
Fleet configs live under the project-root `fleets/` directory. Commands
launched from nested directories discover that outer project root and keep
generated `icp.yaml`, `.icp/`, and `.canic/` state there.

List saved fleet configs:

```bash
canic fleet list
canic fleet delete demo
canic fleet list --network ic
```

Create a new root-plus-app fleet:

```bash
canic fleet create my_app --yes
canic install my_app
```

Diagnose the named fleet, replica reachability, saved config path, and root
readiness:

```bash
canic medic test
```

Run command-specific help when you need exact flags:

```bash
canic <command> help
```

The installed CLI version is visible in top-level help and from `canic
--version`. The version flag is accepted at any command depth, so `canic backup
verify --version` reports the binary version instead of running the command.

## Happy Path

Create a topology-aware backup:

```bash
canic backup create test --dry-run
canic backup create test --subtree app --out backups/<run-id>
```

Non-dry-run captures recompute the selected topology immediately before
snapshot creation and fail if the topology hash changed since discovery. This
keeps subtree backups from silently crossing a registry change.

ICP CLI creates snapshots only for stopped canisters. Canic stops selected
members, creates snapshots, restarts them, downloads artifacts, verifies
checksums, and writes manifest/journal state under the backup directory.

Verify the captured backup directory:

```bash
canic backup verify \
  --dir backups/<run-id>
```

Verification is no-mutation. It validates the manifest, journal agreement,
durable artifact paths, and checksums before restore planning.

## Backup Checks

Use these commands after capture and before restore planning:

- `canic manifest validate` checks manifest shape, topology hash inputs,
  and backup units.
- `canic backup status` summarizes resumable download journal progress.
- `canic backup verify` validates the backup layout and artifact checksums.

For deeper no-mutation restore checks, use `canic restore plan`,
`canic restore apply --dry-run`, and `canic restore run --dry-run` directly.

## Restore Planning

Restore starts from a manifest, not from loose snapshot files:

```bash
canic restore plan \
  --backup-dir backups/<run-id> \
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
canic restore apply \
  --plan restore-plan.json \
  --backup-dir backups/<run-id> \
  --dry-run \
  --out restore-apply-dry-run.json \
  --journal-out restore-apply-journal.json
```

`restore apply` currently requires `--dry-run`; direct mutation through that
command is intentionally disabled. The generated journal is the input to the
guarded runner.

## Guarded Runner

Preview the maintained runner path without calling `icp`:

```bash
canic restore run \
  --journal restore-apply-journal.json \
  --dry-run \
  --network local \
  --out restore-run-dry-run.json
```

Execute a cautious one-step batch:

```bash
canic restore run \
  --journal restore-apply-journal.json \
  --execute \
  --network local \
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
  --journal restore-apply-journal.json \
  --unclaim-pending \
  --out restore-run-recovery.json
```

If an operation failed and you have inspected the failure, move it back to
ready before rerunning execution:

```bash
canic restore run \
  --journal restore-apply-journal.json \
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
