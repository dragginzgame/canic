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
