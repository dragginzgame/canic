# Operations And Diagnostics

The `canic` binary is the operator surface for local workspace setup, builds,
network trust, Fleet installation, inspection, diagnostics, backup, restore,
and recovery. Human-readable output and stable JSON modes serve interactive
and automated workflows without merging their authority.

## What It Provides

- App creation, role scaffolding, attachment, and configuration inspection
- canonical network enrollment and local replica lifecycle
- Fleet installation from explicit placement and funding input
- `status`, `info`, catalog, and environment inspection
- workspace and Fleet `medic` checks with CI and JSON modes
- deployment evidence and passive policy gates
- recovery guidance for retries, receipts, and ambiguous external effects
- local-only verification/import of path-confined, content-addressed
  fresh-install recovery bundles

Useful orientation commands are:

```bash
canic help
canic status
canic medic --ci
```

## Boundary

The CLI may read and write workspace/operator state and invoke the installed
`icp` binary. Live canisters never gain filesystem or identity-key access.
Inspection and dry-run commands remain passive; mutations are exposed as
explicit install, lifecycle, backup, restore, or funding operations.

App, Fleet, and workspace are distinct terms and must not be conflated.

## Start Here

- [Installing Canic](../../../INSTALLING.md)
- [CLI guide](../../../crates/canic-cli/README.md)
- [Operations index](../../operations/README.md)
- [Release validation matrix](../../operations/release-validation-matrix.md)
- [Recovery and retry runbooks](../../operations/recovery-retry-runbooks.md)
- [Supported platforms](../../governance/supported-platforms.md)
