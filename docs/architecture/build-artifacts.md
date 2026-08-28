# Build Artifacts

Canic builds deterministic Wasm and optional binary init arguments for current
desired-state Fleet convergence. Artifact construction is independent of live
Fleet mutation.

## Build

```bash
canic build <app> <role> --profile release \
  --provenance artifacts/<role>-provenance.json
```

The role must belong to the selected App configuration. The builder records
the exact package, profile, input fingerprint, canonical Wasm digest and
deterministic gzip digest. Release builds run the checksum-bound Binaryen 108
`wasm-opt -Oz` transform after shrink and optional public-Candid embedding but
before the code-limit check, gzip, artifact hashes, release-set manifests,
Wasm Store publication, and module-hash authority. The optimized bytes are the
only release artifact; there is no unoptimized fallback or parallel artifact.
Debug and fast builds record that optimization was not requested.

Install the governed optimizer from any published Canic CLI without a source
checkout:

```bash
canic toolchain install
```

The command verifies both the official archive and the extracted executable,
installs `wasm-opt` under `~/.local/bin`, and prints its absolute path. A
release build resolves the first `wasm-opt` on `PATH` and admits it only when
both its exact Binaryen identity and platform-specific executable SHA-256
match Canic's pins. Failure names that selected path and the repair command;
it never searches past a rejected executable or emits unoptimized bytes.

Before replacing the staged input, the release transform derives the required
Wasm feature flags from the module under Canic's admitted IC feature contract
and proves exact export-inventory, feature, and embedded public-Candid parity.
Its provenance records the exact optimizer version and executable SHA-256 plus
before/after raw, deterministic-gzip, code-section, data-section, and defined-
function measurements. The separately materialized Candid and its protocol-
profile digest remain bound before optimization. The builder keeps Wasm
compilation non-incremental. An explicit
`RUSTC_WRAPPER` wins; otherwise an executable `sccache` on `PATH` is used.

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
