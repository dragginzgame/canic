# canic-cli

Operator CLI for Canic backup and restore workflows.

The initial command focuses on snapshot capture/download planning and execution
for a canister plus its registry-discovered children.

```bash
canic snapshot download \
  --canister <canister-id> \
  --root <root-canister-id> \
  --include-children \
  --out backups/<run-id> \
  --dry-run
```

Use `--recursive` instead of `--include-children` to include all descendants.
Use `--registry-json <file>` to plan from a saved `canic_subnet_registry`
response instead of querying a live root. Non-dry-run captures recompute the
selection topology immediately before snapshot creation and fail if the hash
changed since discovery.

DFX only creates snapshots for stopped canisters. Pass
`--stop-before-snapshot --resume-after-snapshot` when the CLI should perform
that local lifecycle step around each captured artifact.

Successful non-dry-run captures write the canonical backup layout: manifest,
download journal, and durable artifact directories. Generated manifests include
each durable artifact checksum so verification can detect manifest/journal
drift before restore planning. Download journals also include
`operation_metrics` counters for target count, snapshot create, snapshot
download, checksum verification, and artifact finalization progress.

Validate a captured manifest before restore planning:

```bash
canic manifest validate \
  --manifest backups/<run-id>/manifest.json \
  --out manifest-validation.json
```

The validation summary includes topology hash inputs, consistency mode, backup
unit counts, kind counts, and per-unit topology validation metadata.

Inspect resumable journal status:

```bash
canic backup status \
  --dir backups/<run-id> \
  --out backup-status.json \
  --require-complete
```

`--require-complete` still writes the JSON status report, then exits with an
error when any artifact has resume work remaining.

Inspect manifest and journal agreement without reading artifact bytes:

```bash
canic backup inspect \
  --dir backups/<run-id> \
  --out backup-inspection.json \
  --require-ready
```

`--require-ready` still writes the JSON inspection report, then exits with an
error when manifest and journal metadata, including topology receipts, are not
ready for full verification.

Emit a provenance report for audit/review workflows:

```bash
canic backup provenance \
  --dir backups/<run-id> \
  --out backup-provenance.json \
  --require-consistent
```

The report records source/tool metadata, topology receipts, declared backup
units, and each member's snapshot/code/artifact provenance without reading
artifact bytes. `--require-consistent` still writes the JSON report, then exits
with an error when manifest and journal backup IDs or topology receipts drift.

Verify the backup layout and durable artifact checksums:

```bash
canic backup verify \
  --dir backups/<run-id> \
  --out backup-integrity.json
```

Run the standard no-mutation preflight bundle:

```bash
canic backup preflight \
  --dir backups/<run-id> \
  --out-dir preflight/<run-id> \
  --mapping restore-map.json \
  --require-restore-ready
```

Preflight writes `manifest-validation.json`, `backup-status.json`,
`backup-inspection.json`, `backup-provenance.json`, `backup-integrity.json`,
`restore-plan.json`, `restore-status.json`, and `preflight-summary.json`.
The summary records the backup ID, source root, environment, topology hash,
readiness statuses, provenance consistency status, topology mismatch count,
journal operation metrics, member counts, restore identity/snapshot/
verification/operation/ordering counts, snapshot provenance readiness booleans,
verification readiness booleans, `restore_mapping_supplied`,
`restore_all_sources_mapped`, `restore_ready`, stable
`restore_readiness_reasons`, and paths to the generated reports.
`--require-restore-ready` still writes the full report bundle, then exits with
an error when `restore_ready` is false.

Restore planning is manifest-driven and performs no mutations:

```bash
canic restore plan \
  --backup-dir backups/<run-id> \
  --mapping restore-map.json \
  --out restore-plan.json \
  --require-verified \
  --require-restore-ready
```

`--require-verified` runs the same manifest, journal, durable artifact, and
checksum checks as `canic backup verify` before emitting the plan.
`--require-restore-ready` still writes the restore plan, then exits with an
error when `readiness_summary.ready` is false.
Restore plans include an `identity_summary` with explicit mapping mode,
all-sources-mapped status, and fixed, relocatable, mapped, in-place, and
remapped member counts. They also include a `snapshot_summary` with module
hash, wasm hash, code version, and checksum coverage counts and readiness
booleans, plus a `verification_summary` with post-restore check counts,
`verification_required`, and `all_members_have_checks`. A `readiness_summary`
collapses those signals into a single `ready` flag and stable reason strings.
Plans also include an `operation_summary` with planned snapshot loads, code
reinstalls, verification checks, and phases, plus an `ordering_summary` and
per-member ordering dependency metadata so dry-runs show when parent
relationships are satisfied inside the same restore group or by an earlier
group.

Emit the initial restore execution status from a plan:

```bash
canic restore status \
  --plan restore-plan.json \
  --out restore-status.json
```

Restore status is no-mutation. It copies the plan identity, readiness,
verification, phase, and operation counts, then marks each planned member as
`planned` with its source/target canister, snapshot ID, and artifact path.

Render the restore execution operations without mutating targets:

```bash
canic restore apply \
  --plan restore-plan.json \
  --status restore-status.json \
  --backup-dir backups/<run-id> \
  --dry-run \
  --out restore-apply-dry-run.json \
  --journal-out restore-apply-journal.json
```

Apply dry-run output expands the restore phases into ordered upload, load,
reinstall, and member verification operations. When `--backup-dir` is supplied,
the dry-run also verifies that referenced artifact paths stay under that backup
directory, exist on disk, and match their expected SHA-256 checksums when the
plan includes checksums. When `--journal-out` is supplied, the command also
writes an initial apply journal with each operation marked `ready` or `blocked`
and stable blocking reasons. The command requires `--dry-run`; real restore
execution is intentionally not enabled yet.

Summarize a restore apply journal:

```bash
canic restore apply-status \
  --journal restore-apply-journal.json \
  --out restore-apply-status.json \
  --require-ready \
  --require-no-pending \
  --require-no-failed \
  --require-complete
```

Use `--require-ready` when scripts should stop if the journal still has blocked
restore operations. Use `--require-no-pending` when scripts should stop if a
restore operation is already claimed and needs inspection or `apply-unclaim`.
Use `--require-no-failed` when failed operations should stop the runner before
completion checks. Use `--require-complete` when scripts should fail until every
apply operation is completed.

Write an operator-focused restore apply report:

```bash
canic restore apply-report \
  --journal restore-apply-journal.json \
  --out restore-apply-report.json \
  --require-no-attention
```

Apply reports include one high-level outcome, attention-required status,
operation counts, blocked reasons, the next transitionable operation, and the
pending, failed, and blocked operation rows that need review. Use
`--require-no-attention` when CI should fail after writing the report if the
journal has pending, failed, or blocked work.

Preview or run the maintained restore runner path:

```bash
canic restore run \
  --journal restore-apply-journal.json \
  --dry-run \
  --network local \
  --out restore-run-dry-run.json
```

Use `restore run --dry-run` to preview the native runner path from the apply
journal. It emits the current status, attention summary, next transition, and
next `dfx` command without mutating the journal or calling `dfx`.

```bash
canic restore run \
  --journal restore-apply-journal.json \
  --execute \
  --network local \
  --max-steps 1 \
  --out restore-run.json \
  --require-run-mode execute \
  --require-stopped-reason max-steps-reached \
  --require-next-action rerun \
  --require-executed-count 1 \
  --require-no-attention
```

Use `restore run --execute` to let `canic` own the guarded runner loop. It
checks readiness, claims the next sequence, runs the generated `dfx` command,
marks the operation completed or failed, and persists the journal after each
transition. `--max-steps` is useful for cautious incremental restores. Add
`--require-complete` or `--require-no-attention` when CI should write the run
summary and then fail if the journal is incomplete or still needs review.
If a generated command fails, the runner still writes the summary and updated
journal before returning a nonzero error.
Every runner summary includes `run_mode`, `stopped_reason`, and `next_action`
so automation can decide whether to rerun, inspect a failed operation, recover
a pending operation, fix blocked inputs, or stop.
Use `--require-run-mode <text>`, `--require-stopped-reason <text>`, and
`--require-next-action <text>` when CI should write the summary and then fail
unless the runner stopped in the expected state.
Use `--require-executed-count <n>` when a batched run must execute exactly the
expected number of operations.

```bash
canic restore run \
  --journal restore-apply-journal.json \
  --unclaim-pending \
  --out restore-run-recovery.json
```

Use `restore run --unclaim-pending` after an interrupted runner leaves one
operation pending. It moves the pending operation back to ready, writes the
updated journal, and emits a recovery summary.

```bash
scripts/restore/apply_journal.sh \
  --journal restore-apply-journal.json \
  --execute \
  --network local \
  --out restore-run.json
```

The script remains as a small compatibility wrapper around
`canic restore run`. It defaults to `--execute` for existing callers, can also
pass through `--dry-run` and `--unclaim-pending`, and accepts the older
`--report-out`, `--status-out`, and `--command-out` flags for existing runbooks.
The native runner now owns the guarded status, claim, `dfx` execution, marking,
and summary behavior.

Emit the full next transitionable operation for an external runner:

```bash
canic restore apply-next \
  --journal restore-apply-journal.json \
  --out restore-apply-next.json
```

Preview the `dfx` command for the next transitionable operation without
executing it:

```bash
canic restore apply-command \
  --journal restore-apply-journal.json \
  --network local \
  --out restore-apply-command.json \
  --require-command
```

Use `--dfx <path>` when the runner should preview a non-default `dfx` binary.
Use `--require-command` when scripts should fail after writing the preview if no
executable operation is available.

Claim the next operation before executing it in an external runner:

```bash
canic restore apply-claim \
  --journal restore-apply-journal.json \
  --sequence 0 \
  --updated-at 2026-05-04T12:00:00Z \
  --out restore-apply-journal.json
```

Claiming marks the next ready operation `pending`. Pending operations remain the
next transitionable operation until `apply-mark` records them as completed or
failed, which lets interrupted runners resume from the same journal. Use
`--sequence <n>` to fail if the next transitionable operation no longer matches
the operation the runner previewed. Use `--updated-at <text>` to record a
runner-provided state marker; when omitted, the CLI writes `unknown`.

Release the current pending operation back to ready when a runner stopped before
executing it:

```bash
canic restore apply-unclaim \
  --journal restore-apply-journal.json \
  --sequence 0 \
  --updated-at 2026-05-04T12:01:00Z \
  --out restore-apply-journal.json
```

Use `--sequence <n>` with `apply-unclaim` when recovery scripts should only
release the pending operation they inspected.

Mark one journal operation after an external restore step completes or fails:

```bash
canic restore apply-mark \
  --journal restore-apply-journal.json \
  --sequence 0 \
  --state completed \
  --updated-at 2026-05-04T12:02:00Z \
  --out restore-apply-journal.json \
  --require-pending
```

Use `--state failed --reason <text>` to record a failed operation. The command
validates the input journal, refuses to skip earlier ready operations, refreshes
operation counts, and writes the updated journal without executing any restore
mutation. Use `--require-pending` when runners should only mark operations that
were claimed first.

Example external restore runner loop:

```bash
set -euo pipefail

journal=restore-apply-journal.json
network=local

while true; do
  canic restore apply-status \
    --journal "$journal" \
    --out restore-apply-status.json \
    --require-ready \
    --require-no-pending \
    --require-no-failed

  if canic restore apply-status \
    --journal "$journal" \
    --out restore-apply-status.json \
    --require-complete; then
    break
  fi

  canic restore apply-command \
    --journal "$journal" \
    --network "$network" \
    --out restore-apply-command.json \
    --require-command

  sequence="$(jq -r '.operation.sequence' restore-apply-command.json)"
  command="$(jq -r '[.command.program] + .command.args | @sh' restore-apply-command.json)"
  updated_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

  canic restore apply-claim \
    --journal "$journal" \
    --sequence "$sequence" \
    --updated-at "$updated_at" \
    --out "$journal"

  eval "$command"

  canic restore apply-mark \
    --journal "$journal" \
    --sequence "$sequence" \
    --state completed \
    --updated-at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --out "$journal" \
    --require-pending
done
```

If the runner stops after claiming work but before executing the previewed
command, inspect `restore-apply-status.json` and use `apply-unclaim --sequence
<n>` to release the pending operation back to `ready`.
