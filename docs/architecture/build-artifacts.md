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

### Final size reporting

Application, infrastructure and selected-role build tables show the selected
Cargo profile and exact `CODE (B)` and `DATA (B)` section payload lengths beside
the existing raw/gzip size summary. These measurements come from the finalized
Wasm through the same host parser used during finalization, including for fast
builds without a Binaryen transform. Section lengths include their internal
encoding; raw size also includes other sections and the module envelope. Data
bytes are not a measure of eventual stable memory, and gzip is transport size.
The table introduces no headroom estimate or new installation limit.

Selected-role `--provenance` evidence records the exact final measurements in
`payload.final_wasm_metrics`: `raw_bytes`, `gzip_bytes`, `code_section_bytes`,
`data_section_bytes` and `defined_functions`. The existing envelope target and
Cargo record identify the profile, while artifact records retain the raw/gzip
hashes. Final metrics are separate from optional transform before/after metrics.
The current v1 payload requires this field through a pre-1.0 hard cut.

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

Repeated Cargo dependency records reuse the first observation of a path within
that snapshot, including paths already captured by package scans. The next
snapshot reads the bytes again. No input observation is cached across the
pre-build/post-build boundary or between build invocations.

Before compilation, retained Cargo build-script output also contributes the paths
exported as `CANIC_CONFIG_SOURCE_PATH`, `CANIC_CONFIG_MODEL_PATH` and
`CANIC_ROLE_RUNTIME_AUTHORITY_PATH`. This discovers already-named external bytes
even when a role's `.d` file is missing. It does not approve an external path first
emitted during the build. Runtime, declaration and explicitly selected intermediate
output roots remain generated outputs; their authored sources and generators stay
in the source snapshot. A catalogue edited before invocation may therefore
regenerate an `OUT_DIR` include without being mistaken for an in-flight source edit.
Existing paths and selected output roots are resolved before classification. An
alias into the selected output tree remains generated, while `target/../source`
does not hide an authored file. Missing paths containing unresolved parent
traversal are tracked rather than granted an output exemption.

The invocation retains file-level source evidence as well as configuration,
environment and tool identity. Replaced Cargo records may stop naming unchanged
inputs; those files and directories are rechecked before accepting the refreshed
inventory. The cache record uses the verified final inventory's digest so the
next invocation can find it. Real edits, additions, deletions and unsupported
file types still refuse recording. An external input first observed after
compilation also refuses recording: its earlier bytes cannot be proved. Typed
diagnostics distinguish a changed input from an unobserved input and include
the affected path.

When this post-build comparison rejects, Canic attempts to retain
`.canic/build-reuse/rejected-<release-build-id>.json` and prints the path on stderr.
This optional record is limited to 256 KiB. It contains the release identity,
rejection kind, before/after snapshot fingerprints and input counts, invocation
paths, selected/resolved output roots and the affected input's available snapshot
values. Values are hashes or the `directory`/`absent` markers. A null input value
means that inventory did not name the path, not that the file was missing; a
dropped path may have been freshly rechecked outside the final inventory. A null
resolved root means canonicalization was unavailable. Source contents and
environment values are omitted. This is failure evidence, never cache authority:
an unavailable destination cannot replace the original error, and corrupt
diagnostics cannot invalidate a verified hit. Successful retries preserve the
record; another rejection for the same release identity replaces it.

Cache metadata lives under `.canic/build-reuse`; release manifests and artifact
bytes remain under the existing `.canic/release-builds/<id>` owner. Cache hits do
not compile, link, optimize or compress Wasm. Corrupt output evidence is rejected
and a new build is selected.

Complete-build misses distinguish unavailable comparison evidence, changed
source/dependency inputs, environment or toolchain/configuration, and rejected
output. The optional `last-input-diagnostics.json` compares against the last
recorded successful build. It never supplies cache authority. Missing, corrupt
or unwritable diagnostic evidence cannot invalidate an otherwise verified hit.
The diagnostic record stores aggregate fingerprints, safe environment key names
and optional HMAC-SHA256 equality tags. Reports show up to eight added/removed
names and eight changed-value names; they never show values or tags. No raw values
or unkeyed individual value hashes are persisted. A random 32-byte comparison key
lives separately at `.canic/local-secrets/build-environment.key`, atomically created
with owner-only permissions under the existing build lock. Keep this key private
and out of exported evidence: possession of both key and tags permits guessing
values. These tags are diagnostic metadata, not credentials or cache authority.

Value attribution covers at most 256 environment entries and safe names of at
most 80 ASCII alphanumeric/underscore characters. Above that bound, or when the
key is missing, corrupt, linked, not owner-only, changed between builds or the
platform cannot safely store it, value attribution is unavailable. Builds and
verified cache hits remain usable; an existing unsafe key is never overwritten.
Removing the key loses comparison continuity. Every inherited build-environment
entry still participates in the real cache identity after the explicit build
environment normalization below.
An attributed launcher key is evidence to investigate at its owner, not permission
to exclude it from build identity.

Build commands remove `CANIC_ICP_IDENTITY_PASSWORD_FILE`, `CODEX_SESSION_ID` and
`CODEX_THREAD_ID` from their inherited environment. Cargo (including metadata and build scripts), compiler/cache probes,
Candid extraction, Wasm transformations, provenance commands and build-tool
acquisition share this boundary. Complete-build and Candid-extraction identities
and reuse diagnostics exclude those same exact keys and bind the compiled
environment policy. Changing or removing the deployment credential or either
session correlation ID therefore does not by itself invalidate reuse. The policy
change requires one initial miss; custom build scripts cannot use these withheld
session IDs as artifact inputs. Canic also sets `SHLVL=0` on every build/tool child and
fingerprints that same value, so shell nesting cannot select another release.
Child shells may increment their own depth normally; custom build scripts must
not use the launcher's shell nesting as an artifact input. This normalization
covers commands as well as cache keys. All other environment values remain bound,
including Make's variables, CI and sandbox configuration, and unknown `CODEX_*`
keys. There is no prefix-based exclusion. Deployment commands retain their existing
identity-unlocking behavior.

This separates an inherited deployment setting from compilation; it is not a
hermetic filesystem or process sandbox. Explicit Cargo configuration and authored
build scripts remain build inputs. Exact selected-release and output verification,
post-build source checks and rejection of unobserved dependencies remain required.

The existing exclusive complete-build reuse lock remains held through lookup,
compilation and finalization. Contention reports progress after one second and
every five seconds thereafter. Stderr reports lock acquisition separately;
input/output verification time excludes it. These are phase observations, not
evidence that lock waiting caused an earlier slow build.

Every runtime embeds the complete release identity. Changed inputs therefore
still rebuild those runtimes for the new identity; this surface does not compose
a new release from artifacts embedding different identities. Reuse never grants
authority to resume or change a Fleet operation.

For isolated release checkouts, keep real, independent `.canic` directories and
Cargo target/build directories. Do not symlink `.canic`, share mutable operation
receipts, or transplant Cargo output as if its recorded absolute paths were
portable. Copied build-script output can still name generated files in the
original checkout. Those files are external inputs to the new checkout and
retain the before/after verification boundary. A normal first build excludes
outputs under its own resolved target roots; a generated filename alone does
not qualify an external path for that exclusion.

Build context diagnostics show selected and physically resolved Cargo/runtime
and declaration output roots before compilation. Resolution includes existing
symlinked parents when output subdirectories have not yet been created. A path
inside another Cargo workspace's `target` emits an advisory explaining that
independent workspace locks do not protect shared mutable output. Explicit
`CARGO_TARGET_DIR` settings remain supported; a dedicated external directory
is not treated as shared without evidence. This bounded check cannot discover
arbitrary external directories shared by unrelated processes, and does not
prove which input caused an earlier cache rejection.

Canic's generated-source writer rejects a symlink at the output file before
reading matching bytes or writing changed bytes. The diagnostic names that
file and recommends an independent target/build directory. This does not
rewrite existing Cargo metadata or certify copied targets as portable; ordinary
regular-file output repair and unchanged timestamps remain supported.

Share the compiler cache through the existing `sccache` wrapper instead. Explicit
`RUSTC_WRAPPER` selection remains authoritative. This does not share Fleet state,
transfer finalized artifacts between release identities, or guarantee a cache
hit across different absolute source paths. If an unobserved-input diagnostic
names another checkout, inspect copied Cargo records and build-script output
before repeating the entire build; do not bypass source verification. Parent
symlinks remain rejected by the existing regular-path persistence boundary.

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
the compiled extraction implementation and build environment described above. The result's byte hash
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
separately measured LLVM LTO duration. Long Cargo children report a heartbeat every
30 seconds with the declaration/runtime phase, batch index/total, bounded role
names, child elapsed time and elapsed time across that phase's compatible batches.
The phase clock continues across successive children; bootstrap and standalone
builds identify their single batch. Captured child output and exit status remain
unchanged. A heartbeat proves a pending child, not CPU activity or progress past
Cargo's internal lock. It includes no raw command or environment values. Known npm `ic-wasm` distribution launchers
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
