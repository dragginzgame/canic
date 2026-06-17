# Blob Storage Gateway Protocol Inventory

Status: **Incomplete - implementation blocked**

Release line: 0.68

Last updated: 2026-06-17

## Purpose

This inventory is the source-of-truth gate for Canic's 0.68 `blob-storage`
protocol work.

No endpoint, DTO, Candid snapshot, macro, stable-record, ops, workflow, or
internal `BlobStorageApi` implementation may merge from this design until this
inventory is complete and cites exact upstream protocol sources.

The 0.68 design requires inventory coverage for all six gateway methods, even
though only four non-billing methods are emitted in 0.68.

## Current Finding

The protocol source has not yet been identified in this repository.

Initial exact-match searches in the local tree found only Canic design notes for:

- `_immutableObjectStorageBlobsAreLive`
- `_immutableObjectStorageBlobsToDelete`
- `_immutableObjectStorageConfirmBlobDeletion`
- `_immutableObjectStorageCreateCertificate`
- `_immutableObjectStorageUpdateGatewayPrincipals`
- `_immutableObjectStorageFundFromProjectCycles`

No Candid signature, source repository URL, source commit SHA, deployed `.did`,
or upstream implementation file has been recorded yet.

## Completion Criteria

This inventory is complete only when every required field below is filled from
an upstream source, generated Candid artifact, deployed interface, or other
maintainer-approved protocol source.

Required source metadata:

- Source repository URL or local source identifier.
- Source commit SHA or immutable provenance identifier.
- Per-method source file path.
- Per-method source file commit SHA when different from the repository SHA.
- Generated Candid source path or command used to generate the Candid.
- Production gateway identifiers for reference only.
- Production Cashier identifier for reference only.

Required behavior metadata:

- Method name.
- Query/update mode.
- Exact Candid signature.
- Request DTO shape.
- Response DTO shape.
- Nested records and variants.
- Result/error variant behavior.
- Trap, reject, and result behavior.
- Unauthorized behavior.
- Malformed request behavior.
- Exact external input encoding.
- Batch ordering semantics.
- Duplicate-input semantics.
- Idempotency expectations.
- Production-vs-local behavior differences.
- 0.68 or 0.69 ownership classification.
- Toko behavior-level compatibility notes.

### Status Vocabulary

Method status values are intentionally narrow:

- `Missing source`: no immutable protocol source has been identified.
- `Source identified`: source repository or local source identifier, immutable
  provenance, and per-method source path are recorded, but Candid or behavior
  fields remain incomplete.
- `Snapshot captured`: source metadata and exact Candid are recorded, but
  behavior fields or compatibility notes remain incomplete.
- `Complete`: every required source, Candid, behavior, and compatibility field
  is filled from cited protocol evidence.

Design-note statements may describe expected ownership or implementation
direction, but they do not satisfy source, Candid, DTO, behavior, or
compatibility fields. Keep unknown protocol facts as `TBD` instead of inferring
them from the 0.68 design.

## Method Inventory

Every method section must keep design-only facts separate from source-backed
facts. Do not move a method out of `Missing source` until at least the source
identifier, immutable provenance, per-method source path, and generated or
deployed Candid source are recorded.

### `_immutableObjectStorageBlobsAreLive`

Status: **Missing source**

Owning release: 0.68

Emission in 0.68: yes

Known from design only:

- Non-billing liveness query backed by 0.68 live blob state.
- Public/malformed behavior must match the upstream protocol.
- External input encoding is unresolved. Implementers must not assume text
  hashes or `sha256:<hex>` unless the source proves that shape.

Required fields:

- Source repository or local source identifier: TBD
- Source commit SHA: TBD
- Source file path: TBD
- Mode: TBD
- Candid signature: TBD
- Request DTO shape: TBD
- Response DTO shape: TBD
- Malformed input behavior: TBD
- Unauthorized behavior: TBD
- Batch ordering semantics: TBD
- Duplicate-input semantics: TBD
- Absent-hash behavior: TBD
- Maximum batch size: TBD
- Production-vs-local differences: TBD

### `_immutableObjectStorageBlobsToDelete`

Status: **Missing source**

Owning release: 0.68

Emission in 0.68: yes

Known from design only:

- Non-billing deletion coordination backed by 0.68 pending deletion state.
- Caller authorization must be gateway-only against stored gateway principals.
- Non-gateway behavior is unresolved and must not be invented.

Required fields:

- Source repository or local source identifier: TBD
- Source commit SHA: TBD
- Source file path: TBD
- Mode: TBD
- Candid signature: TBD
- Request DTO shape: TBD
- Response DTO shape: TBD
- Unauthorized behavior: TBD
- Result ordering: TBD
- Maximum batch size: TBD
- Repeat-return behavior until confirmation: TBD
- Empty pending-deletion behavior: TBD
- Production-vs-local differences: TBD

### `_immutableObjectStorageConfirmBlobDeletion`

Status: **Missing source**

Owning release: 0.68

Emission in 0.68: yes

Known from design only:

- Non-billing deletion confirmation backed by 0.68 lifecycle transitions.
- Caller authorization must be gateway-only against stored gateway principals.
- Unknown, already-confirmed, and live-but-not-pending behavior is unresolved.

Required fields:

- Source repository or local source identifier: TBD
- Source commit SHA: TBD
- Source file path: TBD
- Mode: TBD
- Candid signature: TBD
- Request DTO shape: TBD
- Response DTO shape: TBD
- Unauthorized behavior: TBD
- Unknown blob behavior: TBD
- Live-but-not-pending behavior: TBD
- Already-confirmed behavior: TBD
- Idempotency semantics: TBD
- Production-vs-local differences: TBD

### `_immutableObjectStorageCreateCertificate`

Status: **Missing source**

Owning release: 0.68

Emission in 0.68: yes

Known from design only:

- Non-billing registration/certificate protocol entrypoint.
- Macro `guard = <access expression>` protects this endpoint.
- Certificate material source and mutation ordering are unresolved and must come
  from the protocol source.

Required fields:

- Source repository or local source identifier: TBD
- Source commit SHA: TBD
- Source file path: TBD
- Mode: TBD
- Candid signature: TBD
- Request DTO shape: TBD
- Response DTO shape: TBD
- Certificate material source: TBD
- Mutation-before-certificate behavior: TBD
- Rollback or no-rollback behavior: TBD
- Repeated create behavior: TBD
- Metadata conflict/enrichment behavior: TBD
- Unauthorized behavior: TBD
- Malformed request behavior: TBD
- Production-vs-local differences: TBD

### `_immutableObjectStorageUpdateGatewayPrincipals`

Status: **Missing source**

Owning release: 0.69

Emission in 0.68: no

Known from design only:

- Deferred billing/sync endpoint.
- 0.68 must inventory the exact signature so 0.69 consumes, rather than
  invents, the gateway-facing protocol.

Required fields:

- Source repository or local source identifier: TBD
- Source commit SHA: TBD
- Source file path: TBD
- Mode: TBD
- Candid signature: TBD
- Request DTO shape: TBD
- Response DTO shape: TBD
- Unauthorized behavior: TBD
- Cashier dependency: TBD
- Production-vs-local differences: TBD

### `_immutableObjectStorageFundFromProjectCycles`

Status: **Missing source**

Owning release: 0.69

Emission in 0.68: no

Known from design only:

- Deferred billing/funding endpoint.
- 0.68 must inventory the exact signature so 0.69 consumes, rather than
  invents, the gateway-facing protocol.

Required fields:

- Source repository or local source identifier: TBD
- Source commit SHA: TBD
- Source file path: TBD
- Mode: TBD
- Candid signature: TBD
- Request DTO shape: TBD
- Response DTO shape: TBD
- Cycle attachment requirements: TBD
- Unauthorized behavior: TBD
- Funding success/failure behavior: TBD
- Production-vs-local differences: TBD

## Compatibility Notes

### Toko

Status: **Incomplete**

Required before implementation:

- Existing Toko live blob state shape.
- Existing Toko pending deletion state shape.
- Existing Toko gateway-principal state shape.
- Mapping from Toko blob identity into Canic `BlobRootHash`.
- Whether Toko can bulk-register state, read through existing state, move state,
  or start with empty state.

No 0.68 storage schema should be treated as final until these compatibility
facts are recorded or an explicit no-state-move path is accepted.

## Implementation Gate

The following actions are blocked while this document remains incomplete:

- Adding the `blob-storage` feature.
- Adding gateway DTOs.
- Adding Candid snapshots.
- Emitting `_immutableObjectStorage*` endpoints.
- Adding blob storage stable records.
- Adding blob storage ops or workflow modules.
- Adding `BlobStorageApi`.
- Adding PocketIC lifecycle tests that assert protocol behavior.

This gate is enforced in CI and local Make test/release-bump paths by
`scripts/ci/check-blob-storage-inventory-gate.sh`. While the status remains
incomplete, the guard rejects blob-storage feature metadata, source/module
paths, gateway method literals, and public blob-storage API/model names outside
this protocol inventory/design documentation. When this inventory is marked
`Complete`, the same guard verifies that all six method sections are present
and individually complete, have no `TBD` fields, and that the Toko
compatibility section is also complete.

The only safe next steps are:

- Locate the upstream source or generated `.did`.
- Fill the method inventory from immutable source references.
- Add Candid snapshots copied or generated from the inventoried source.
- Update the 0.68 design if the protocol source contradicts current design
  assumptions.
