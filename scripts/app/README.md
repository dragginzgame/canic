# Local Demo Workflow (`scripts/app/`)

These scripts support the reference canisters under `fleets/test/` and the local topology in `icp.yaml`.

## Prerequisites

- Canic/Rust tooling installed:
  - `make install-dev`
  - or `bash scripts/dev/install_dev.sh`
  - the shared setup script bootstraps Rust when needed, checks Python 3, installs the pinned internal Rust toolchain, `rustfmt`, `clippy`, `wasm32-unknown-unknown`, `candid-extractor`, `ic-wasm`, common cargo helper tools, the matching `canic` CLI, and `icp` if it is missing
  - the same script also configures `.githooks/` automatically when run from a Canic checkout

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

## Install the Reference Topology

From the repo root:

```bash
make test-fleet-install
```

Canic now supports three wasm build profiles:
- `debug`: plain Cargo debug wasm, mainly for raw artifact/debugging work
- `fast`: the middle local/test profile, smaller and faster than debug without paying full release cost
- `release`: the shipping/install profile

`make test-fleet-install` and `make test-canisters` default to the middle `fast`
profile, and `CANIC_WASM_PROFILE` is the explicit selector.

If you want to force release wasm artifacts for the same flow, run:

```bash
CANIC_WASM_PROFILE=release make test-fleet-install
```

If you want the raw debug wasm lane instead, run:

```bash
CANIC_WASM_PROFILE=debug make test-fleet-install
```

This one command:
- creates the reference canisters in `icp`
- builds the local canister artifacts
- emits the build-produced root staging manifest from the configured ordinary `.wasm.gz` artifacts
- reinstalls `root` in `Prime` mode
- stages the configured ordinary fleet artifacts into `root` through the install flow
- resumes bootstrap so `root` can create the internal `wasm_store` and publish the staged artifacts
- waits for `root` to report `READY`

This is a manual local fast flow, not part of `make test`.

`make test` now runs with `--nocapture`, so long serial PocketIC runs keep
their phase markers visible by default:

```bash
make test
```

The fast non-PocketIC path follows the same rule:

```bash
make test-wasm
```

## Build Canisters

From the repo root:

```bash
icp canister create --all
icp build --all
```

`icp.yaml` uses custom build commands which call `scripts/app/build.sh <canister>`. That script:
- is now just a thin wrapper around `canic build <canister>`
- prints the workspace/ICP roots once per `icp build` parent process and a short elapsed-time line per canister build so long downstream/custom-build runs stay readable

That public builder:
- builds the requested Rust canister crate for `wasm32-unknown-unknown`
- refreshes the implicit bootstrap `wasm_store` artifact automatically when building `root`
- keeps `wasm_store` out of downstream `icp.yaml` and delegates the implicit bootstrap build through the Canic backend builder
- lets the public bootstrap builder resolve the canonical `canic-wasm-store` source automatically from the current `canic` checkout or published registry source, and if that canonical crate is not present it synthesizes a wrapper directly from the resolved `canic` source, so downstreams do not need their own `wasm_store` crate or extra `wasm_store` build config
- copies the resulting WASM into `.icp/local/canisters/<name>/<name>.wasm`
- runs `candid-extractor` to produce `.icp/local/canisters/<name>/<name>.did`

The visible reference canister `.did` files now live only under `.icp/local`.
They are generated build artifacts, not committed source files.

The one checked-in exception is:
- `crates/canic-wasm-store/wasm_store.did`

That file remains the canonical published interface for the implicit bootstrap
`wasm_store` crate and the packaged downstream CLI path.

Ordinary bootstrap builds copy that checked-in DID into `.icp/local`; they do
not rewrite the checked-in source file unless
`CANIC_REFRESH_WASM_STORE_DID=1` is set intentionally.

Profile selection for the public builder is:
- `CANIC_WASM_PROFILE=debug|fast|release`

## Why `.wasm.gz` Exists

`icp.yaml` sets `"gzip": true`, so icp 0.30.2 also writes a gzipped artifact:
`.icp/local/canisters/<name>/<name>.wasm.gz`.

`root.wasm` stays thin again. Only the bootstrap `wasm_store.wasm.gz` is
embedded in `root`; the ordinary role `.wasm.gz` artifacts stay outside `root`
and are staged after `root` install from the build-produced
`.icp/local/canisters/root/root.release-set.json` manifest by `canic install`.

During normal custom builds, `scripts/app/build.sh` now opportunistically emits
that manifest as soon as the full root-subnet ordinary artifact set exists, so
downstreams do not need a local copy of the manifest-emission logic just to
keep `.icp/local/canisters/root/root.release-set.json` in sync.

If you do not want the repo-local wrapper at all, use the `canic` CLI directly:

```bash
canic build root
canic install test root
```

In split repos where the Rust workspace lives under `backend/` but `icp.yaml`
and `.icp` live at the repo root, set:

```bash
CANIC_WORKSPACE_ROOT=/path/to/repo/backend
CANIC_ICP_ROOT=/path/to/repo
```

The first root drives Cargo and config discovery; the second root owns emitted
artifacts and the generated bootstrap-store wrapper.

If canister crates live under a different directory such as
`backend/src/canisters`, also set:

```bash
CANIC_CANISTERS_ROOT=src/canisters
```

relative to `CANIC_WORKSPACE_ROOT`, or point `CANIC_CONFIG_PATH` at the real
`canic.toml` path and let the builder infer the canister root from that config
location.

The builder also tries Cargo workspace metadata first, so nested paths like
`src/canisters/project/ledger` work without extra config when package names
still follow `canister_<role>`. If a package name does not follow that
convention, declare the mapping in `Cargo.toml`:

```toml
[package.metadata.canic]
role = "project_ledger"
```
