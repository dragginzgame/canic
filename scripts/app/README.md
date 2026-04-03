# Local Demo Workflow (`scripts/app/`)

These scripts support the reference canisters under `canisters/` and the local topology in `dfx.json`.

## Prerequisites

- Canic/Rust tooling installed:
  - `make install-all`
  - or `make install-canister-deps`
  - or `bash scripts/install.sh`
  - the shared setup script bootstraps Rust when needed, installs the pinned Rust toolchain, `rustfmt`, `clippy`, `wasm32-unknown-unknown`, `candid-extractor`, `ic-wasm`, common cargo helper tools, the matching `canic-installer`, and `dfx` if it is missing
  - the same script also configures `.githooks/` automatically when run from a Canic checkout

## Local Replica Contract

The install/build commands below assume the target `dfx` replica is already
running. They fail fast if it is not.

If you want a manual convenience helper for local work, use:

```bash
scripts/app/dfx_start.sh
```

That helper is optional and repo-local only. The install/test flows still do
not auto-start `dfx`.

## Install the Reference Topology

From the repo root:

```bash
make demo-install
```

Canic now supports three wasm build profiles:
- `debug`: plain Cargo debug wasm, mainly for raw artifact/debugging work
- `fast`: the middle local/test profile, smaller and faster than debug without paying full release cost
- `release`: the shipping/install profile

`make demo-install` and `make test-canisters` default to the middle `fast`
profile, and `CANIC_WASM_PROFILE` is the explicit selector.

If you want to force release wasm artifacts for the same flow, run:

```bash
CANIC_WASM_PROFILE=release make demo-install
```

If you want the raw debug wasm lane instead, run:

```bash
CANIC_WASM_PROFILE=debug make demo-install
```

This one command:
- creates the reference canisters in `dfx`
- builds the local canister artifacts
- emits a build-produced root release-set manifest from the configured ordinary `.wasm.gz` artifacts
- reinstalls `root` in `Prime` mode
- stages the configured ordinary release set into `root` through the published Rust helper in `canic-installer`
- resumes bootstrap so `root` can create the internal `wasm_store` and publish the staged release set
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
dfx canister create --all
dfx build --all
```

`dfx.json` uses custom build commands which call `scripts/app/build.sh <canister>`. That script:
- is now just a thin wrapper around the published `canic-build-canister-artifact` binary from `canic-installer`
- prints the workspace/DFX roots once per `dfx build` parent process and a short elapsed-time line per canister build so long downstream/custom-build runs stay readable

That public builder:
- builds the requested Rust canister crate for `wasm32-unknown-unknown`
- refreshes the hidden bootstrap `wasm_store` artifact automatically when building `root`
- keeps `wasm_store` out of downstream `dfx.json` and delegates the hidden bootstrap build to the published `canic-build-wasm-store-artifact` tool from `canic-installer`
- lets the public bootstrap builder resolve the canonical `canic-wasm-store` source automatically from the current `canic` checkout or published registry source, and if that canonical crate is not present it synthesizes a hidden wrapper directly from the resolved `canic` source, so downstreams do not need their own `wasm_store` crate or extra `wasm_store` build config
- copies the resulting WASM into `.dfx/local/canisters/<name>/<name>.wasm`
- runs `candid-extractor` to produce `.dfx/local/canisters/<name>/<name>.did`

The visible reference canister `.did` files now live only under `.dfx/local`.
They are generated build artifacts, not committed source files.

The one checked-in exception is:
- `crates/canic-wasm-store/wasm_store.did`

That file remains the canonical published interface for the hidden bootstrap
`wasm_store` crate and the packaged downstream installer path.

Profile selection for the public builder is:
- `CANIC_WASM_PROFILE=debug|fast|release`

## Why `.wasm.gz` Exists

`dfx.json` sets `"gzip": true`, so dfx 0.30.2 also writes a gzipped artifact:
`.dfx/local/canisters/<name>/<name>.wasm.gz`.

`root.wasm` stays thin again. Only the bootstrap `wasm_store.wasm.gz` is
embedded in `root`; the ordinary role `.wasm.gz` artifacts stay outside `root`
and are staged after `root` install from the build-produced
`.dfx/local/canisters/root/root.release-set.json` manifest by the Rust helpers
in `canic-installer`.

During normal custom builds, `scripts/app/build.sh` now opportunistically emits
that manifest as soon as the full root-subnet ordinary artifact set exists, so
downstreams do not need a local copy of the manifest-emission logic just to
keep `.dfx/local/canisters/root/root.release-set.json` in sync.

If you do not want the repo-local wrapper at all, the equivalent direct calls are:

```bash
scripts/app/canic_installer.sh canic-build-canister-artifact root
scripts/app/canic_installer.sh canic-install-root root
```

In split repos where the Rust workspace lives under `backend/` but `dfx.json`
and `.dfx` live at the repo root, set:

```bash
CANIC_WORKSPACE_ROOT=/path/to/repo/backend
CANIC_DFX_ROOT=/path/to/repo
```

The first root drives Cargo and config discovery; the second root owns emitted
artifacts and the hidden generated bootstrap-store wrapper.

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
