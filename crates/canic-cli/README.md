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
drift before restore planning.

Validate a captured manifest before restore planning:

```bash
canic manifest validate \
  --manifest backups/<run-id>/manifest.json \
  --out manifest-validation.json
```

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
  --mapping restore-map.json
```

Preflight writes `manifest-validation.json`, `backup-status.json`,
`backup-inspection.json`, `backup-provenance.json`, `backup-integrity.json`,
`restore-plan.json`, and `preflight-summary.json`.
The summary records the backup ID, source root, environment, topology hash,
readiness statuses, provenance consistency status, topology mismatch count,
member counts, and paths to the generated reports.

Restore planning is manifest-driven and performs no mutations:

```bash
canic restore plan \
  --backup-dir backups/<run-id> \
  --mapping restore-map.json \
  --out restore-plan.json \
  --require-verified
```

`--require-verified` runs the same manifest, journal, durable artifact, and
checksum checks as `canic backup verify` before emitting the plan.
