# Build Artifacts

Canic builds deterministic Wasm and optional binary init arguments for current
desired-state Fleet convergence. Artifact construction is independent of live
Fleet mutation.

## Build

```bash
canic build <app> <role> --profile release \
  --provenance artifacts/<role>-provenance.json
```

The role must belong to the selected App configuration. Before Cargo starts,
the builder resolves one exact `ic-wasm 0.11.1` executable for every profile
and one checksum-bound Binaryen 132 executable for release. Every artifact in
that invocation reuses those absolute paths. The builder records the exact
package, profile, input fingerprint, canonical Wasm digest and deterministic
gzip digest. Release builds run `wasm-opt -Oz` after shrink and optional
public-Candid embedding but before the code-limit check, gzip, artifact hashes,
release-set manifests, Wasm Store publication, and module-hash authority. The
optimized bytes are the only release artifact; there is no unoptimized fallback
or parallel artifact. Debug and fast builds record that optimization was not
requested. A missing transform tool is a build failure, not a provenance
outcome.

Install both governed Wasm tools from any published Canic CLI without a source
checkout:

```bash
canic toolchain install
```

The command verifies both official archives, verifies the extracted tool
identities, installs `ic-wasm` and `wasm-opt` under `~/.local/bin`, and prints
both absolute paths. Builds prefer those canonical installed paths and fall
back to PATH only when a canonical executable is absent. `ic-wasm` must report
the exact pinned version. Binaryen must match both its exact version identity
and platform-specific executable SHA-256. Failure names the selected or missing
canonical path and the repair command. An executable found and rejected at an
authoritative path is never skipped in favor of another candidate.

Before replacing the staged input, the release transform derives the required
Wasm feature flags from the module under Canic's admitted IC feature contract
and proves exact export-inventory, feature, and embedded public-Candid parity.
Its provenance records the exact optimizer version and executable SHA-256 plus
before/after raw, deterministic-gzip, code-section, data-section, and defined-
function measurements. The separately materialized Candid and its protocol-
profile digest remain bound before optimization. The builder keeps Wasm
compilation non-incremental. An explicit
`RUSTC_WRAPPER` wins; otherwise an executable `sccache` on `PATH` is used.

### Complete build reuse and compilation phases

`canic build <app>` verifies inputs before allocating another release identity.
An unchanged complete build returns its original finalized release after checking
all retained artifact and manifest bytes. Source, configuration, lockfile,
environment, compiler/target-library and admitted tool changes invalidate reuse.
The input set includes Cargo-declared target files, the exact family source roots
selected by generated infrastructure, and recorded dependency paths, including
shared build scripts and includes outside package directories. Infrastructure
sources are included before compilation even when their features are absent
from the App's ordinary Cargo graph. Build scripts must declare external inputs
to Cargo. Input collection is conservative across the complete Cargo catalog,
rather than a minimal per-role dependency cache.

The invocation retains file-level source evidence as well as configuration,
environment and tool identity. Replaced Cargo records may stop naming unchanged
inputs; those files and directories are rechecked before accepting the refreshed
inventory. The cache record uses the verified final inventory's digest so the
next invocation can find it. Real edits, additions, deletions and unsupported
file types still refuse recording. An external input first observed after
compilation also refuses recording: its earlier bytes cannot be proved. Typed
diagnostics distinguish a changed input from an unobserved input and include
the affected path.
Cache metadata lives under `.canic/build-reuse`; release manifests and artifact
bytes remain under the existing `.canic/release-builds/<id>` owner. Cache hits do
not compile, link, optimize or compress Wasm. Corrupt output evidence is rejected
and a new build is selected.

Every runtime embeds the complete release identity. Changed inputs therefore
still rebuild those runtimes for the new identity; this surface does not compose
a new release from artifacts embedding different identities. Reuse never grants
authority to resume or change a Fleet operation.

Configured declarations use the selected profile with optimization level zero,
LTO explicitly off and 16 codegen units in a separate `declarations` Cargo
target. They omit the release nonce and retain exact features, configuration
and build network. Runtime compilation batches compatible packages by workspace
with an exact package/role protocol-digest context. Combined Cargo resolution must preserve each package's isolated
normal/build dependency tree and feature sets; conflicting packages split into
separate batches. Canonical Coordinator and Store sidecars avoid ordinary
declaration builds. Production runtime LTO remains the existing release policy.

After Cargo validates declaration dependencies, unchanged declaration Wasm can
reuse its normalized Candid extraction across new complete release identities.
The host keys this optional result by exact Wasm bytes, native extractor bytes,
the compiled extraction implementation and environment. The result's byte hash
and input bindings are checked on every hit. A changed role misses when its
compiled declaration changes; an unaffected role may hit. Shared inputs remain
Cargo dependencies and changed compiled outputs miss independently.

This cache lives under `.canic/build-reuse/declarations`. Corrupt or oversized
records cause ordinary extraction, and inability to retain a cache record does
not reject an otherwise successful extraction. The cache I/O size bound does
not impose a Candid size limit. Extractor or Wasm changes during extraction
refuse the result. Current role capabilities, protocol-profile hashes and
runtime outputs are still derived afterward; no finalized runtime is copied
between release identities. Stderr distinguishes each role's declaration hit
or miss from its complete-release cache decision.

Complete App builds hold one artifact-build lock across compilation and
finalization. Immediately after each Coordinator or Store compilation, the host
captures that exact Wasm in a private staging directory. Up to two infrastructure
workers run the existing finalizers while subsequent Cargo commands execute
serially. Scoped workers finish and discard their captured inputs before the
build returns, including when a later step fails. Single-role builds stay
synchronous. Release manifests are sealed only after all requested outputs pass
qualification; a failed build may leave qualified individual artifacts, but
cannot return a successful complete build. Infrastructure elapsed times can
overlap each other and configured-role time; they must not be added to infer
total build wall time.

Stderr reports each role's cache decision and the observed compilation and
finalization phases. Runtime Cargo/link time includes linking; it is not a
separately measured LLVM LTO duration. Known npm `ic-wasm` distribution launchers
resolve to their native executable before admission and hashing. An unrecognized
scripted tool cannot establish a cache hit; ordinary compilation remains usable.

### Standalone-local runtime

A canister using `canic::start_local!` can select its local-only Cargo surface
without recreating Canic's declaration build:

```bash
canic build <app> <role> --standalone-local \
  --features standalone-local --profile fast
```

`--features` accepts a comma-separated set and may be paired with
`--no-default-features`. Canic applies the exact same sorted feature selection
to the declaration and runtime passes. The first pass exists only to produce
the adjacent `<role>.did`; its Wasm is never published. The final runtime must
omit both `get_candid_pointer` and public `candid:service` metadata, and its
exported query/update method inventory must exactly match the sidecar before
the artifact is returned. When ICP CLI sets `ICP_WASM_OUTPUT_PATH`, Canic copies
that sidecar-only runtime to the requested output path. Use ICP CLI's
`--candid <role>.did` option when a later command needs interface decoding.

Build and Fleet environments are separate concepts. A deterministic artifact
may be built locally and later referenced by a desired document for another
selected network. The desired Fleet plan binds the artifact bytes actually
present in the operator workspace.

## Plan Binding

Each `canic fleet ensure` plan resolves and hashes:

- every configured raw Wasm;
- every binary init-argument file;
- every retirement Candid file.

The immutable reviewed plan contains those identities. Apply re-reads each file
immediately before its effect and rejects a missing, non-regular, changed or
digest-mismatched artifact. It never substitutes a similarly named build or
loads an artifact from a historical release bundle.

## Current State Boundary

The current reconciler persists its plan and effect journal under
`.canic/fleet-ensure/<environment>/<fleet>/`. It does not read historical
finalized-release manifests, deployment plans, repair receipts or recovery
bundles as Fleet authority. Those records may remain as immutable historical
evidence but do not influence current convergence.

After a successful creation, the current identity map retains the returned
Principal for immediate and interrupted reruns. The desired document should
record stable operator-reviewed Principals for long-lived estates so a lost
local workspace cannot hide a cycle-bearing canister.

## Safety

Artifact equality authorizes code bytes only. It does not authorize funding,
controllers, placement, replacement or deletion. Those decisions remain in
the reviewed Fleet plan and are constrained by the cycle-conservation and
retirement rules documented in
[Fleet ensure](../features/operations/fleet-ensure.md).
