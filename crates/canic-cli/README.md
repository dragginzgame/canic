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
  --out restore-apply-status.json
```

Emit the full next ready operation for an external runner:

```bash
canic restore apply-next \
  --journal restore-apply-journal.json \
  --out restore-apply-next.json
```

Preview the `dfx` command for the next ready operation without executing it:

```bash
canic restore apply-command \
  --journal restore-apply-journal.json \
  --network local \
  --out restore-apply-command.json
```

Use `--dfx <path>` when the runner should preview a non-default `dfx` binary.

Mark one journal operation after an external restore step completes or fails:

```bash
canic restore apply-mark \
  --journal restore-apply-journal.json \
  --sequence 0 \
  --state completed \
  --out restore-apply-journal.json
```

Use `--state failed --reason <text>` to record a failed operation. The command
validates the input journal, refuses to skip earlier ready operations, refreshes
operation counts, and writes the updated journal without executing any restore
mutation.
