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
response instead of querying a live root.

DFX only creates snapshots for stopped canisters. Pass
`--stop-before-snapshot --resume-after-snapshot` when the CLI should perform
that local lifecycle step around each captured artifact.

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

Verify the backup layout and durable artifact checksums:

```bash
canic backup verify \
  --dir backups/<run-id> \
  --out backup-integrity.json
```

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
