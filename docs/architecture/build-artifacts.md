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
the exact package, profile, input fingerprint, raw Wasm digest and deterministic
gzip digest. It keeps Wasm compilation non-incremental. An explicit
`RUSTC_WRAPPER` wins; otherwise an executable `sccache` on `PATH` is used.

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
