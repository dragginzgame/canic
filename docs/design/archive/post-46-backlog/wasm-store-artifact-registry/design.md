# Post-46 Backlog: Wasm Store Artifact Registry And Retention

## Status

Backlog only.

This is a post-0.46 backlog topic. It is intentionally outside the approved
0.41 through 0.46 deployment truth foundation and is not a promised numbered
follow-on release.

---

## Goal

A future line may make `wasm_store` a provenance-rich artifact registry with
safe retention behavior.

Core operator question:

```text
Which artifacts are present, why are they retained, what provenance do they
carry, and what can be collected safely?
```

---

## Dependency On Completed Deployment Foundation

0.41 through 0.46 make `wasm_store` observable deployment infrastructure:

- visible in plan, inventory, artifact manifest, and diff;
- typed as both `ArtifactRole::WasmStore` and possible
  `ArtifactTransport::WasmStore`;
- digest-checked;
- receipted through staging;
- comparable as artifact availability evidence.

This topic builds on those facts. It should not backfill missing deployment
truth semantics into the store.

---

## Core Decision

`wasm_store` may become a provenance-rich artifact registry, but it remains
artifact evidence.

It does not become deployment truth or deployment authority.

Live inventory remains the source of truth for installed canisters, module
hashes, controllers, and trust-domain state.

---

## Data Model

### ArtifactAddressV1

```text
ArtifactAddressV1 {
  digest,
  digest_algorithm,
  payload_kind,
}
```

`payload_kind` distinguishes sealed wasm, wasm.gz, Candid, manifest, metadata,
and other registry payloads.

### ArtifactRegistryEntryV1

```text
ArtifactRegistryEntryV1 {
  address,
  role,
  sealed_wasm_sha256,
  wasm_gz_sha256,
  candid_sha256,
  canonical_embedded_config_sha256,
  source_build_identity,
  provenance,
  created_at,
  size_bytes,
}
```

The entry describes artifact identity. It does not assert that the artifact is
currently installed.

### ArtifactProvenanceV1

```text
ArtifactProvenanceV1 {
  source_kind,
  source_revision,
  package_name,
  package_version,
  builder_version,
  rust_toolchain,
  build_profile,
  build_environment_digest,
}
```

Provenance is useful for audit and rollback selection. It is not a substitute
for artifact digests.

### ArtifactPinV1

```text
ArtifactPinV1 {
  artifact_address,
  pin_kind,
  pinned_by,
  deployment_id,
  receipt_id,
  expires_at,
}
```

Pins explain retention. Receipt pins and deployment pins should be explicit so
operators can see why an artifact cannot be collected.

### RetentionPolicyV1

```text
RetentionPolicyV1 {
  keep_receipt_pinned,
  keep_deployment_pinned,
  keep_latest_per_role,
  rollback_window,
  max_store_bytes,
}
```

Retention policy must never delete artifacts still required by active
deployments, verified rollback policy, or explicit pins.

### WasmStoreGcPlanV1

```text
WasmStoreGcPlanV1 {
  candidates,
  protected_by_pins,
  reclaimable_bytes,
  hard_failures,
}
```

GC is plan-first. Apply should execute only a plan whose candidates still match
the current store inventory.

---

## Registry Behavior

The registry should support digest-addressed lookup alongside existing
template/version addressing during migration.

Registry behavior should answer:

- which digest-addressed artifacts exist;
- which role and source/build identity produced them;
- which canonical embedded config they carry;
- which receipt or deployment references them;
- whether a target deployment already has a needed artifact staged.

Cross-deployment availability reports are allowed. Implicit cross-trust-domain
promotion is not.

---

## Retention And GC

Retention and GC should be conservative:

- collect only from an explicit `WasmStoreGcPlanV1`;
- re-read current store state before apply;
- protect receipt-pinned and deployment-pinned artifacts;
- protect the configured rollback window;
- report blocked candidates instead of forcing deletion;
- receipt every applied deletion and verified postcondition.

Migration from template/version addressing to digest-addressed artifacts should
produce a report before it mutates store metadata.

---

## Commands

Tentative operator surface:

```text
canic deploy artifacts list <deployment>
canic deploy artifacts inspect <artifact>
canic deploy artifacts pin <artifact>
canic deploy artifacts gc plan <deployment>
canic deploy artifacts gc apply <plan>
```

Command names are tentative. The stable part is the registry and retention
model.

---

## Non-Goals

This future work should not:

- make `wasm_store` deployment truth;
- make `wasm_store` controller authority;
- be required for 0.41 through 0.46 promotion;
- perform implicit cross-trust-domain promotion;
- choose rollback targets without an explicit policy;
- hide live inventory mismatches behind registry metadata.

---

## Implementation Slices

### Slice 1: Registry Metadata Model

Define registry entries, artifact addresses, and provenance payloads.

### Slice 2: Digest-Addressed Lookup

Add digest-addressed lookup alongside template/version addressing.

### Slice 3: Provenance Capture

Record source/build identity and build environment evidence where available.

### Slice 4: Pinning Model

Represent receipt, deployment, operator, and rollback pins.

### Slice 5: GC Planning

Produce conservative GC plans with protected candidates and reclaimable bytes.

### Slice 6: Safe GC Execution

Apply only verified GC plans and receipt all deletions.

### Slice 7: Addressing Migration Report

Report and then migrate from template/version addressing to digest-addressed
metadata where safe.

---

## Exit Criterion

This topic is ready to promote into a real release line when:

```text
Operators can answer which artifacts are present, why they are retained, what
provenance they carry, and what can be collected without risking active
deployments or rollback policy.
```
