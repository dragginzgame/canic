# Local Demo Workflow (`scripts/app/`)

These scripts support the reference canisters under `apps/test/` and the local topology in `icp.yaml`.

## Prerequisites

- Canic/Rust tooling installed:
  - `make install-dev`
  - or `bash scripts/dev/install_dev.sh`
  - the shared setup script requires Rustup/Cargo, checks Python 3, installs the pinned internal Rust toolchain, `rustfmt`, `clippy`, `wasm32-unknown-unknown`, `candid-extractor`, `ic-wasm`, common cargo helper tools, the matching `canic` CLI, and `icp`

## Local Replica Contract

The local install commands below now auto-restart a clean local `icp` replica
once when `icp ping local` fails. Nonlocal targets still fail fast and expect
their target replica to be managed externally.

If you want a manual convenience command for local work, use:

```bash
canic replica start
canic replica status
```

The local install/test flows can recover the local `icp` replica themselves.
To detach the replica, run `canic replica start --background`.

## Exercise The Current Install Boundary

From the repository root, provide an operator-owned Fleet input with exact
local placement, admission, limit, and funding policy:

```bash
canic install test test-local \
  --fleet-input <path> \
  --profile fast
```

Canic now supports three wasm build profiles:

- `debug`: plain Cargo debug wasm, mainly for raw artifact/debugging work
- `fast`: the middle local/test profile, smaller and faster than debug without paying full release cost
- `release`: the shipping/install profile

If you want to force release wasm artifacts for the same flow, run:

```bash
canic install test test-local --fleet-input <path> --profile release
```

If you want the raw debug wasm lane instead, run:

```bash
canic build test app --profile debug
```

The current 0.100 flow:

- compiles the complete Component Topology and immutable multi-root install
  plan before effects
- builds separate Coordinator, Fleet Subnet Root, and Wasm Store
  infrastructure artifacts
- builds one Fleet-wide application artifact union and exact admitted release
  set per root
- creates, installs, and independently verifies the Coordinator and every
  planned root
- stages each root's release set and verifies exactly one root-local Store
- registers and independently verifies every root as Registry `Joining`
- stops explicitly before snapshot synchronization, acknowledgement, root
  activation, Component creation, and terminal Fleet-catalog publication

The stop is the current implementation boundary. It is not a ready reference
topology, and terminal `canic info`, application calls, backup, and restore are
not valid follow-up steps yet.

## Build Canisters

From the repo root:

```bash
icp canister create --all
icp build --all
```

This repo's `icp.yaml` uses custom build commands which call the host artifact
builder directly from the checkout:

```bash
CARGO_INCREMENTAL=0 cargo run -q --profile fast -p canic-host --example build_artifact -- \
  <canister> <debug|fast|release> <workspace-root> <icp-root> <config-path>
```

Downstream repos that consume Canic from crates.io should use the installed CLI
surface instead:

```bash
canic build <app> <role>
```

That builder:
- builds the requested Rust canister crate for `wasm32-unknown-unknown`
- keeps `wasm_store` out of downstream `icp.yaml`; normal installation builds
  it as a separately qualified Canic infrastructure artifact
- resolves the canonical `canic-fleet-coordinator` source from the current
  `canic` checkout or published registry source, with a generated runtime-only
  fallback when that package is unavailable
- resolves the canonical `canic-wasm-store` source from the current `canic`
  checkout or published registry source, so downstreams do not need their own
  `wasm_store` crate or extra Store build config
- copies the resulting WASM into `.icp/local/canisters/<role>/<role>.wasm`
- copies the uncompressed WASM to `ICP_WASM_OUTPUT_PATH` when invoked by ICP
  CLI custom builds
- runs `candid-extractor` to produce `.icp/local/canisters/<role>/<role>.did`

The visible reference canister `.did` files now live only under `.icp/local`.
They are generated build artifacts, not committed source files.

The one checked-in Candid exception is:
- `crates/canic-wasm-store/wasm_store.did`

That file remains the canonical published interface for the implicit bootstrap
`wasm_store` crate and the packaged downstream CLI path.

Ordinary bootstrap builds copy that checked-in DID into `.icp/local`; they do
not rewrite the checked-in source file. Maintainers refresh it explicitly with
the low-level builder's `--refresh-wasm-store-did` argument.

Profile selection for the builder is:
- `canic build <app> <role> --profile debug|fast|release` when using the
  installed CLI
- explicit role, profile, workspace-root, ICP-root, and config-path arguments
  for the low-level `build_artifact` example

## Why `.wasm.gz` Exists

`icp.yaml` sets `"gzip": true`, so the pinned ICP CLI also writes a gzipped
artifact: `.icp/local/canisters/<role>/<role>.wasm.gz`.

The Coordinator, Fleet Subnet Root, and Wasm Store `.wasm.gz` files are
qualified as separate infrastructure artifacts. Ordinary Component and
descendant `.wasm.gz` files stay outside the root Wasm. Normal installation
builds their exact Fleet-wide union once, then projects and stages only the
release set admitted to each root.

For normal fleet work, use the fleet-aware installer instead of a per-role
builder command:

```bash
canic install test test-local --fleet-input <path>
```

In split repos where the Rust workspace lives under `backend/` but `icp.yaml`
and `.icp` live at the repo root, pass the roots to the installed CLI:

```bash
canic build --workspace /path/to/repo/backend --icp-root /path/to/repo <app> <role>
```

The first root drives Cargo and config discovery; the second root owns emitted
artifacts and the generated bootstrap-store wrapper.

If canister crates live under a different directory such as
`backend/src/canisters`, point the command at the real config:

```bash
canic build --workspace /path/to/repo/backend --icp-root /path/to/repo --config /path/to/repo/backend/src/canisters/canic.toml <app> <role>
```

The builder infers the canister root from that config location.

The builder also tries Cargo workspace metadata first, so nested paths like
`src/canisters/project/ledger` work without extra config when package names
still follow `canister_<role>`. If a package name does not follow that
convention, declare the mapping in `Cargo.toml`:

```toml
[package.metadata.canic]
app = "project"
role = "project_ledger"
```
