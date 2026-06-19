# Blob Storage Gateway Protocol Inventory

Status: **Incomplete - implementation blocked**

Release line: 0.69

Last updated: 2026-06-17

## Purpose

This inventory is the source-of-truth gate for Canic's 0.69 `blob-storage`
protocol work.

No endpoint, DTO, Candid snapshot, macro, stable-record, ops, workflow, or
internal `BlobStorageApi` implementation may merge from this design until this
inventory is complete and cites exact upstream protocol sources.

The 0.69 design requires inventory coverage for all six gateway methods, even
though only four non-billing methods are emitted in 0.69.

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

## Protocol Source Search Log

### 2026-06-19 Local Workspace Search

Search scope:

- `/home/adam/projects/canic`
- `/home/adam/projects/toko`
- `/home/adam/projects/icydb` Candid artifact paths discovered by local file
  search

Search terms:

```text
_immutableObjectStorage
immutableObjectStorage
BlobRootHash
storage_gateway_principal
account_balance_get
account_top_up
Caffeine
caffeine
```

Candidate interface files searched:

- `*.did`
- `*.candid`
- `dfx.json`
- `canic.toml`

Result:

- No upstream immutable object-storage gateway implementation was found.
- No generated or deployed gateway `.did` was found.
- No source-backed Candid signatures were found for the six
  `_immutableObjectStorage*` methods.
- Matches for `_immutableObjectStorage*`, `BlobRootHash`, Cashier method names,
  and Caffeine wording were limited to Canic design notes, inventory files,
  changelog notes, guard scripts, and inventory-gate tests.
- The only sibling checkout matching the local project search was
  `/home/adam/projects/toko`, which remains useful for first-consumer
  compatibility evidence but does not contain the gateway protocol source or
  method literals.
- Candid files discovered under `/home/adam/projects/icydb/artifacts` are
  unrelated local wasm-size artifacts and are not gateway protocol evidence.

Inventory effect:

- Every gateway method remains `Missing source`.
- No method section may move to `Source identified`, `Snapshot captured`, or
  `Complete` from this local search alone.
- The implementation gate remains closed.

Next required evidence:

- Upstream source repository URL or local source identifier for the immutable
  object-storage gateway, plus immutable commit/provenance.
- Generated or deployed `.did` for the gateway interface.
- Per-method source file path and source-backed behavior evidence.
- Maintainer-approved answer for the Toko `BlobRootHash` mapping: proven
  source mapping, accepted empty-state adoption, accepted bulk registration, or
  accepted external-mapping migration path.

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
- 0.69 or 0.70 ownership classification.
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
them from the 0.69 design.

## Method Inventory

Every method section must keep design-only facts separate from source-backed
facts. Do not move a method out of `Missing source` until at least the source
identifier, immutable provenance, per-method source path, and generated or
deployed Candid source are recorded.

### `_immutableObjectStorageBlobsAreLive`

Status: **Missing source**

Owning release: 0.69

Emission in 0.69: yes

Known from design only:

- Non-billing liveness query backed by 0.69 live blob state.
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

Owning release: 0.69

Emission in 0.69: yes

Known from design only:

- Non-billing deletion coordination backed by 0.69 pending deletion state.
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

Owning release: 0.69

Emission in 0.69: yes

Known from design only:

- Non-billing deletion confirmation backed by 0.69 lifecycle transitions.
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

Owning release: 0.69

Emission in 0.69: yes

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

Owning release: 0.70

Emission in 0.69: no

Known from design only:

- Deferred billing/sync endpoint.
- 0.69 must inventory the exact signature so 0.70 consumes, rather than
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

Owning release: 0.70

Emission in 0.69: no

Known from design only:

- Deferred billing/funding endpoint.
- 0.69 must inventory the exact signature so 0.70 consumes, rather than
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

Status: **Incomplete - local Toko source captured, blob-root mapping unresolved**

Source evidence captured:

- Local source identifier: sibling checkout `../toko`
- Source commit SHA: `600dcfbe91c30311c5896f3ac0399d27e2e36ab6`
- Source checkout note: the local checkout had Cargo metadata edits, but the
  source files cited below were not listed as dirty by `git status --short`.

Existing Toko asset-canister source state:

- `backend/src/design/src/entity/asset.rs` defines source-of-truth uploaded
  binary state as `Asset` records in `schema::asset::store::AssetStore`.
  `Asset` stores a storage-native ULID `id`, unbounded text `reference`,
  `creator_pid`, `Manifest`, and file `Metadata`.
- The same file defines uploaded `Chunk` records in
  `schema::asset::store::ChunkStore`, keyed by a storage-native ULID `id` with
  `asset_id`, `index`, unbounded `bytes`, and an optional 32-byte `sha256`.
- `backend/src/design/src/app/asset/mod.rs` defines `Manifest` as chunk-upload
  state: expected chunk count/size, received chunk count/bytes, completion
  bool, and a chunk-present bitmap.
- `backend/src/design/src/app/asset/file.rs` defines file `Metadata` with an
  optional `hash`, but `fleets/toko/asset/src/dto/mod.rs` currently converts
  `PostRemoteAsset` into metadata without setting a file hash.
- `fleets/toko/asset/src/ops/chunk.rs` verifies each uploaded chunk against the
  caller-supplied 32-byte SHA-256 before inserting it. This is per-chunk
  integrity, not proof of a whole-blob root hash.

Existing Toko project-instance mirror state:

- `backend/src/design/src/entity/project/instance/asset.rs` defines
  `RemoteAsset` as a project-instance projection with `id`, `name`, `location`,
  `reference`, `tags`, `mime_type`, `extension`, and optional `thumbnail`.
- `Location` stores the source asset-canister ULID and the asset-canister
  principal. The local creation path in
  `fleets/toko/project/instance/src/ops/asset.rs` constructs a new
  `RemoteAsset` from the source `Asset` and stores source identity in
  `location.asset_id`.
- Local source comments say `Id<RemoteAsset>` is externally asserted from the
  asset-canister ULID, but the observed creation path inserts a fresh
  `RemoteAsset` without assigning the source asset id as the row id. The 0.69
  Canic design must treat `location.asset_id` as the reliable observed source
  identity unless Toko is corrected or a later audit proves otherwise.

Existing Toko deletion state:

- `fleets/toko/project/instance/src/ops/asset.rs` prevents deleting remote
  assets assigned to live tokens, calls the asset canister `delete_assets`, then
  deletes project-local `RemoteAsset` rows.
- `fleets/toko/asset/src/ops/asset.rs` handles `delete_assets` by deleting
  `Asset` and `Chunk` rows immediately. No pending-deletion queue,
  gateway-confirmation state, tombstone, or gateway-principal check was found
  in the local Toko source.

Compatibility findings:

- Existing Toko live asset state is chunk-upload state, not the Caffeine
  immutable object-storage gateway protocol.
- Existing Toko pending deletion state shape: none found.
- Existing Toko gateway-principal state shape: none found.
- Mapping from Toko blob identity into Canic `BlobRootHash`: unresolved. Toko
  currently has asset ULIDs, text references, optional unset file metadata
  hashes, and per-chunk SHA-256 values. None of those are proven to be the
  gateway's canonical blob root hash.
- Migration/read-through strategy: unresolved. A compatible path may require
  empty-state adoption, explicit bulk registration, or an external source that
  maps existing Toko asset identities to canonical gateway blob roots. This is
  still TBD and must be decided before implementation unlocks.

No 0.69 storage schema should be treated as final until these compatibility
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
- Adding `blob-storage-billing`, Cashier wrappers, gateway-principal sync,
  funding, or status surfaces that depend on blob-storage implementation.

This gate is enforced in CI and local Make test/release-bump paths by
`scripts/ci/check-blob-storage-inventory-gate.sh`. While the status remains
incomplete, the guard rejects blob-storage feature metadata, source/module
paths, gateway method literals, public blob-storage API/model names, and
premature blob-storage billing/Cashier implementation surfaces outside this
protocol inventory/design documentation. When this inventory is marked
`Complete`, the same guard verifies that all six method sections are present and
individually complete, have no `TBD` fields, include required common and
method-specific evidence labels, reject placeholder field values, validate
method source commit SHA shape, and confirm that the Toko compatibility section
is also complete with local source, commit, blob-root mapping, and
migration/read-through strategy evidence.

The only safe next steps are:

- Locate the upstream source or generated `.did`.
- Fill the method inventory from immutable source references.
- Add Candid snapshots copied or generated from the inventoried source.
- Update the 0.69 design if the protocol source contradicts current design
  assumptions.
