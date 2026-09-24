# Release-binding feasibility checkpoint

Date: 2026-09-24. Scope: isolated synthetic Wasm, not a production runtime cut.
The [proposal](../../../../design/ideas/release-binding-finalization/design.md)
remains unscheduled. The operator progress correction interrupted this experiment
and was completed before the experiment resumed.

## Observed result

One Rust `no_std` module was compiled with release optimization, fat LTO and one
codegen unit, then processed by the pinned `ic-wasm shrink` and Binaryen `-Oz`
tools using Canic's supported feature flags. The resulting template is 684 bytes.
An exact named immutable global exports the address of a 64-byte binding slot.
The finalizer resolves that address through parsed Wasm globals and active data
segments, with module validation and bounds/overlap checks; it never searches
for a release string in arbitrary bytes.

The optimized template finalized to two distinct identities/hashes. Exactly the
64 bytes at file offset 620 changed; all bytes before and after the slot remained
identical, including executable code and exports. Repeating a finalization
produced identical bytes. A changed template hash, malformed identity and an
already-bound input were rejected.

An owned local PocketIC 16 server then observed both identities through the
fixture's executable query. Each identity survived same-Wasm restoration. The
fixture's init check rejected an unbound template and the other identity's init
argument with typed `CanisterError` responses. Successful installs independently
show that the retained global export is accepted by this simulator. The server
was owned and shut down by the harness; no live IC call or deployment occurred.

The [structured result](release-binding-feasibility/result.json),
[fixture](release-binding-feasibility/fixture.rs),
[harness](release-binding-feasibility/harness.rs),
[isolated manifest](release-binding-feasibility/Cargo.toml),
[lockfile](release-binding-feasibility/Cargo.lock) and
[tool hashes](release-binding-feasibility/tools.sha256) retain the checkpoint.
Local raw/shrunk/optimized/final Wasm, build/server logs, compiler identity,
source inventory and hashes live in `.tmp/release-binding-feasibility-20260924/`.

## Production boundary still to prove

This fixture uses a volatile runtime read and an explicit linker-exported
address. It does not run `canic::start!`, derive identities from reviewed release
nonces, implement Canic activation, publish artifact manifests or change cache
admission. Its simple finalizer deliberately does not implement the full imported
global, passive-data or multi-memory model. Do not reuse it as a product finalizer.

The source review identifies these maintained consumers:

| Owner | Current binding | Required production proof |
| --- | --- | --- |
| Host build context | `CANIC_RELEASE_BUILD_ID` supplied to child compilation | Stable compilation context with exact input qualification and fresh release ownership |
| `canic::build!` | Environment rerun tracking and `rustc-env` forwarding | Remove release-specific compilation invalidation without ignoring arbitrary build inputs |
| `start!` / managed non-root lifecycle | Embedded static plus direct environment values in init and restoration adapters | Every managed-role entrypoint reads the finalized value; no optimized constant remains |
| `start_fleet_root!` | Embedded identity passed to Root init/restoration | Canonical Root, activation and same-release recovery checks use the exact finalized identity |
| `start_wasm_store!` | Embedded static and Store lifecycle values | Store startup and restoration bind the finalized identity |
| `start_fleet_coordinator!` | Embedded static and managed Coordinator lifecycle | Coordinator authority uses the finalized identity |
| Core/control-plane runtime | Canonical parsing and activation binding checks | Existing guards reject mismatched final artifacts without a compatibility lane |

`start_local!` is a separate standalone-development surface; do not accidentally
turn it into a release-set member or weaken managed entrypoint admission.
`ic-wasm shrink` removes custom sections whose names do not start with `icp:`;
a future custom descriptor must account for that pipeline instead of assuming
an arbitrary section survives. The successful probe uses an exported global.

Next evidence must cover the real canonical-role/application matrix, all optimized
identity consumers, exact Candid and metadata extraction, template admission,
atomic finalization, tampering/interruption/retry, and unchanged runtime costs.
Compare controlled unchanged/qualification/gameplay/dependency/relocation builds
before making a build-speed claim. No application-scale timing or resource saving
was measured here, and no runtime/ABI tradeoff has been accepted.

## Reproduction outline

Use the retained compiler/tool identities and the isolated lockfile. Compile
`fixture.rs` as `cdylib` for `wasm32-unknown-unknown` with `opt-level=z`,
`lto=fat`, `codegen-units=1`, `panic=abort`, `strip=symbols`, and linker argument
`--export=CANIC_BINDING_SLOT_V1`. Run `ic-wasm shrink`, then `wasm-opt -Oz` with
`--enable-bulk-memory --enable-sign-ext --enable-nontrapping-float-to-int`.
Name that output `optimized.wasm` in an owned scratch directory. Build this
isolated harness with its locked manifest, then pass the scratch directory and
the checksum-bound PocketIC binary as its two arguments. This is one focused
simulator experiment, not a workspace or release gate.
