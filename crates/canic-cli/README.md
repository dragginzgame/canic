# canic-cli

`canic-cli` publishes the `canic` operator binary. It is the command-line
surface for listing a Canic fleet, capturing canister snapshots, validating
backup artifacts, and preparing guarded restores.

The CLI currently wraps `dfx` for live snapshot and restore mutations. Canic
owns the topology selection, manifests, journals, readiness checks, restore
ordering, and runner state around those `dfx` calls.

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

For a full local development setup, including `dfx`, helper tools,
`canic-cli`, and `canic-installer`, use the install script in the root README.

## First Commands

Show the current registered fleet as an ASCII tree:

```bash
canic list --network local
```

By default, `canic list` resolves the current project's root with
`dfx canister id root`. Use `--root <root-canister-id>` to point at a specific
root, `--canister <id>` to print one subtree, or `--registry-json <file>` to
render a saved `canic_subnet_registry` response without calling `dfx`.

Run command-specific help when you need exact flags:

```bash
canic <command> help
```

Print the installed CLI version with `canic --version`. The flag is accepted
at any command depth, so `canic backup preflight --version` reports the binary
version instead of running the command.

## Happy Path

Capture a canister and its direct registered children:

```bash
canic snapshot download \
  --canister <canister-id> \
  --root <root-canister-id> \
  --include-children \
  --out backups/<run-id>
```

Use `--recursive` instead of `--include-children` to include all descendants.
Use `--dry-run` to compute the target set without creating or downloading
snapshots. Use `--registry-json <file>` to plan from a saved registry response
instead of querying a live root.

Non-dry-run captures recompute the selected topology immediately before
snapshot creation and fail if the topology hash changed since discovery. This
keeps subtree backups from silently crossing a registry change.

`dfx` creates snapshots only for stopped canisters. Pass
`--stop-before-snapshot --resume-after-snapshot` when the CLI should perform
that local lifecycle step around each captured artifact.

Run the standard post-capture smoke wrapper:

```bash
canic backup smoke \
  --dir backups/<run-id> \
  --out-dir smoke/<run-id> \
  --require-design \
  --require-restore-ready
```

Smoke is no-mutation. It writes the preflight report bundle, renders restore
operations, creates a restore apply journal, previews the native runner path,
and records the readiness flags in `smoke-summary.json`.

For the release smoke path, use the canonical checklist:
[docs/operations/0.30-backup-restore-smoke.md](../../docs/operations/0.30-backup-restore-smoke.md).

## Backup Checks

Use these commands after capture and before restore planning:

- `canic manifest validate` checks manifest shape, topology hash inputs,
  backup units, and design conformance.
- `canic backup status` summarizes resumable download journal progress.
- `canic backup inspect` compares manifest and journal metadata without reading
  artifact bytes.
- `canic backup provenance` reports source, topology, unit, member, snapshot,
  code, and artifact provenance.
- `canic backup verify` reads durable artifacts and verifies checksums.
- `canic backup preflight` runs the standard no-mutation validation bundle and
  emits restore planning/status reports.
- `canic backup smoke` runs preflight plus restore dry-run and runner preview.

The stricter flags intentionally write their reports before returning a nonzero
exit code. That lets CI and operators inspect the failure artifact that explains
why a backup is not ready.

## Restore Planning

Restore starts from a manifest, not from loose snapshot files:

```bash
canic restore plan \
  --backup-dir backups/<run-id> \
  --mapping restore-map.json \
  --out restore-plan.json \
  --require-verified \
  --require-design \
  --require-restore-ready
```

Planning performs no mutations. It validates mapping, identity mode, snapshot
provenance, verification coverage, artifact checksums when requested, and
restore ordering. Plans include operation counts and parent-before-child
ordering metadata so operators can see the intended restore sequence before any
target is touched.

Create the initial restore status:

```bash
canic restore status \
  --plan restore-plan.json \
  --out restore-status.json
```

Render operations and create an apply journal:

```bash
canic restore apply \
  --plan restore-plan.json \
  --status restore-status.json \
  --backup-dir backups/<run-id> \
  --dry-run \
  --out restore-apply-dry-run.json \
  --journal-out restore-apply-journal.json
```

`restore apply` currently requires `--dry-run`; direct mutation through that
command is intentionally disabled. The generated journal is the input to the
guarded runner.

## Guarded Runner

Preview the maintained runner path without calling `dfx`:

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
  --updated-at 2026-05-05T12:03:00Z \
  --out restore-run.json \
  --require-no-attention
```

The native runner checks journal readiness, claims the next operation, runs the
generated `dfx` command, marks the operation completed or failed, and persists
the journal after each transition. `--max-steps 1` is the safest operational
mode while validating a new restore path.

If a previous runner stopped after claiming work, release the pending operation
back to ready:

```bash
canic restore run \
  --journal restore-apply-journal.json \
  --unclaim-pending \
  --updated-at 2026-05-05T12:10:00Z \
  --out restore-run-recovery.json
```

## Restore Journal Tools

These commands inspect the journal produced by `restore apply --dry-run`:

- `canic restore apply-status` summarizes progress, blocked work, pending
  claims, failed operations, and completion counts.
- `canic restore apply-report` writes an operator-focused report for the work
  needing attention.

`canic restore run` is the only maintained command for advancing a restore
journal. It owns command preview, claiming, execution, completion/failure
records, and pending-operation recovery.

The compatibility wrapper `scripts/restore/apply_journal.sh` remains available
for older runbooks, but it delegates to `canic restore run`.

## Safety Model

- Directory data may select a root, but topology defines membership.
- Captures fail closed when the selected topology hash changes before snapshot
  creation.
- Backup manifests carry topology, unit, identity, snapshot, artifact,
  provenance, and verification metadata.
- Restore planning is no-mutation and must prove mapping, ordering, checksum,
  verification, and design-conformance readiness before execution.
- Runner summaries and journals are durable audit artifacts; failures still
  write status before returning a nonzero exit code.
