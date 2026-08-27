# Local Demo Workflow

These helpers support `apps/test/` and the local topology in `icp.yaml`.

## Prerequisites

```bash
make install-dev
canic replica start --background
```

The setup installs the pinned Rust/ICP/Wasm/Candid tools and `sccache`.

## Build

```bash
canic build test --profile fast
canic build test app --profile debug
```

The installed CLI is the normal builder. ICP custom builds call the same host
artifact owner from this checkout:

```bash
CARGO_INCREMENTAL=0 cargo run -q --profile fast \
  -p canic-host --example build_artifact -- \
  <canister> <debug|fast|release> \
  <workspace-root> <icp-root> <config-path>
```

Canic builds attached App roles together, then builds canonical infrastructure
artifacts separately. Local App builds generate `.did` files; production builds
omit local Candid metadata.

## Ensure

Create `fleets/test-local.toml` using the current contract in
[Fleet ensure](../../docs/features/operations/fleet-ensure.md).

```bash
canic fleet ensure test-local --desired fleets/test-local.toml
canic fleet ensure test-local \
  --desired fleets/test-local.toml \
  --apply <plan_sha256>
```

The first command has no paid Fleet mutation. Review the exact canister
dispositions and cycle-conservation bounds before applying. Rerun the same
digest after interruption. Historical install/recovery state and commands are
not part of this workflow.

## Split Workspace And ICP Roots

```bash
canic build \
  --workspace /path/to/repo/backend \
  --icp-root /path/to/repo \
  --config /path/to/repo/backend/apps/test/canic.toml \
  test
```

Use absolute explicit paths so Cargo and ICP artifact ownership cannot diverge.
Every managed package should declare exact App/role metadata.
