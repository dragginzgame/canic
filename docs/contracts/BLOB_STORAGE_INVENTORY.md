# Blob Storage Gateway Protocol Inventory

Status: **Incomplete - implementation blocked**

Release line: 0.69

Last updated: 2026-06-19

## Purpose

This inventory is the source-of-truth gate for Canic's 0.69 `blob-storage`
protocol work.

No endpoint, DTO, Candid snapshot, macro, stable-record, ops, workflow, or
internal `BlobStorageApi` implementation may merge from this design until this
inventory is complete and cites exact upstream protocol sources.

The 0.69 design requires inventory coverage for all six gateway methods, even
though only four non-billing methods are emitted in 0.69.

## Current Finding

The local Toko `origin/development` commit
`3ef01afc5f5eeefdb9471f3e010b6562d758c111` now provides inspected,
source-backed Toko project-instance evidence for the
`_immutableObjectStorage*` canister endpoints and generated Toko Candid.

The current Toko `boss` `HEAD` does not contain that blob-storage surface, and
no separate immutable object-storage gateway implementation or deployed gateway
`.did` has been identified.

Initial exact-match searches in the local Canic tree found only design notes
for:

- `_immutableObjectStorageBlobsAreLive`
- `_immutableObjectStorageBlobsToDelete`
- `_immutableObjectStorageConfirmBlobDeletion`
- `_immutableObjectStorageCreateCertificate`
- `_immutableObjectStorageUpdateGatewayPrincipals`
- `_immutableObjectStorageFundFromProjectCycles`

The Toko development-commit evidence is enough to replace the earlier
no-source finding for the project-side canister endpoints. It is not enough by
itself to complete this inventory until the maintainer accepts Toko's
project-instance implementation as the protocol source, the Toko
compatibility/migration answer is resolved, and the billing/Cashier source
remains separated into the 0.70 inventory.

## Protocol Source Search Log

### 2026-06-19 Initial Local Workspace Search

This result is historical. It remains useful for the Canic/local-`boss` search
record, but the later Toko `origin/development` inspection supersedes its
no-source conclusion for the project-instance endpoint source.

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

### 2026-06-19 GitHub Installed Repository Code Search

Search scope:

- GitHub App installed repositories visible to this session:
  `dragginzgame/canic` and `dragginzgame/toko`

Search terms:

```text
_immutableObjectStorage
_immutableObjectStorageBlobsAreLive
_immutableObjectStorageBlobsToDelete
_immutableObjectStorageConfirmBlobDeletion
_immutableObjectStorageCreateCertificate
blob_storage
```

Result:

- No separate Caffeine or immutable object-storage gateway source repository
  was visible through the installed GitHub repositories.
- GitHub code search found newer Toko consumer-side candidates at commit
  `3ef01afc5f5eeefdb9471f3e010b6562d758c111`.
- Candidate project-instance gateway endpoint references were found in:
  - `fleets/toko/project/instance/src/lib.rs`
  - `fleets/toko/project/instance/project_instance.did`
  - `frontend/src/generated/declarations/project_instance/project_instance.did.js`
  - `frontend/src/generated/declarations/project_instance/project_instance.did.d.ts`
- `_immutableObjectStorageCreateCertificate` also matched:
  - `frontend/src/lib/storage/storage-client.ts`
- Candidate project-hub blob-storage status/billing references were found in:
  - `fleets/toko/project/hub/src/ops/blob_storage.rs`
  - `backend/src/canisters/project/hub/src/ops/blob_storage.rs`
  - `fleets/toko/project/hub/project_hub.did`
  - `frontend/src/generated/declarations/project_hub/project_hub.did.js`
  - `frontend/src/generated/declarations/project_hub/project_hub.did.d.ts`

Local inspection limit, superseded by the later local Toko inspection below:

- The local Toko checkout is at
  `600dcfbe91c30311c5896f3ac0399d27e2e36ab6` on branch `boss`, with local
  Cargo metadata edits, and does not contain GitHub-indexed commit
  `3ef01afc5f5eeefdb9471f3e010b6562d758c111`.
- The local `gh` token is invalid in this session, so the indexed private Toko
  files could not be fetched through `gh`.
- The dirty local Toko checkout was not fetched or modified.

Inventory effect:

- These results are useful candidate Toko consumer evidence.
- They are not authoritative gateway protocol source and do not satisfy
  source-backed Candid, DTO, or behavior fields.
- Every gateway method remains `Missing source`.
- The implementation gate remains closed.

Superseded next-step note:

- The Toko candidate commit was later inspected locally; see
  `2026-06-19 Local Toko Development Commit Inspection`.
- Locate any separate immutable object-storage gateway implementation or
  generated/deployed gateway `.did`, if that is required beyond the
  project-instance endpoint source.
- Continue to keep Toko compatibility evidence and Cashier/billing evidence
  separate from the 0.69 project-side endpoint evidence.

### 2026-06-19 Local Toko Development Commit Inspection

Search scope:

- `/home/adam/projects/toko`

Checkout state:

- Checked-out branch: `boss`
- Checked-out `HEAD` commit:
  `97aafee9eeb73ae0517f9788df688bb96ae0a9ff`
- Worktree note: the Toko checkout is user-managed and later showed unmerged
  pull state; do not treat worktree conflict contents as canonical evidence.
- Exact `git grep` on checked-out `HEAD` for `_immutableObjectStorage*`,
  `BlobRootHash`,
  `account_balance_get_v1`, `account_top_up_v1`,
  `storage_gateway_principal_list_v1`, `get_blob_storage_status`, and
  `Cashier` returned no protocol matches.
- Candidate commit
  `3ef01afc5f5eeefdb9471f3e010b6562d758c111` exists locally on
  `origin/development` and was inspected with `git show` / `git grep` without
  checking it out.

Source-backed Toko project-instance evidence at commit
`3ef01afc5f5eeefdb9471f3e010b6562d758c111`:

- `fleets/toko/project/instance/src/lib.rs` exposes all six
  `_immutableObjectStorage*` endpoint methods.
- `fleets/toko/project/instance/src/ops/immutable_storage.rs` implements the
  project-side storage state, liveness, deletion, registration, gateway
  principal sync, and project-cycle funding behavior.
- `fleets/toko/project/instance/project_instance.did` contains generated
  Candid signatures for the project-instance endpoints.
- `backend/src/design/src/entity/project/instance/storage.rs` defines
  `StoredBlob`, `BlobDeletionPending`, `StorageGatewayPrincipal`, and
  `ProjectStorageConfig`.
- `backend/src/design/src/app/asset/blob.rs` defines `BlobRootHash` as a
  content-addressed `sha256:<64-hex>` text value.
- `frontend/src/lib/storage/storage-client.ts` implements the Caffeine gateway
  upload client: chunk hashes use the `icfs-chunk/` domain separator, metadata
  hashes use `icfs-metadata/`, node hashes use `ynode/`, the blob tree type is
  `DSBMTWH`, and uploads target `/v1/blob-tree/`, `/v1/chunk/`, and
  `/v1/blob/`.

Generated Toko Candid signatures captured from
`fleets/toko/project/instance/project_instance.did`:

- `_immutableObjectStorageBlobsAreLive : (vec blob) -> (vec bool) query`
- `_immutableObjectStorageBlobsToDelete : () -> (vec text) query`
- `_immutableObjectStorageConfirmBlobDeletion : (vec blob) -> ()`
- `_immutableObjectStorageCreateCertificate : (text) -> (CreateCertificateResult)`
- `_immutableObjectStorageFundFromProjectCycles : (nat) -> (BlobProjectCyclesTopUpReport)`
- `_immutableObjectStorageUpdateGatewayPrincipals : () -> ()`

Inventory effect:

- The project-side `_immutableObjectStorage*` canister endpoint source and
  generated Toko Candid are now inspected.
- This does not prove the current `boss` branch carries the surface; it does
  not.
- This does not identify a separate gateway service implementation or deployed
  gateway `.did`.
- The inventory remains incomplete until the method sections are completed
  from accepted protocol evidence and the Toko compatibility/migration answer
  is resolved.

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

Status: **Source identified**

Owning release: 0.69

Emission in 0.69: yes

Source-backed Toko evidence:

- Source repository or local source identifier: sibling checkout `../toko`
- Source commit SHA: `3ef01afc5f5eeefdb9471f3e010b6562d758c111`
- Source file path:
  `fleets/toko/project/instance/src/ops/immutable_storage.rs`
- Endpoint file path: `fleets/toko/project/instance/src/lib.rs`
- Generated Candid source path:
  `fleets/toko/project/instance/project_instance.did`
- Mode: query
- Candid signature: `(vec blob) -> (vec bool) query`
- Request DTO shape: `Vec<Vec<u8>>`; each entry is expected to be a 32-byte
  root hash and is converted to `sha256:<64-hex>`.
- Response DTO shape: `Vec<bool>`.
- Malformed input behavior: entries that are not exactly 32 bytes return
  `false`; converted hashes that fail `sha256:<64-hex>` validation return
  `false`.
- Unauthorized behavior: public query; no auth guard in the inspected Toko
  endpoint.
- Batch ordering semantics: response order follows input order.
- Duplicate-input semantics: duplicate hashes are evaluated independently and
  return duplicate booleans.
- Absent-hash behavior: `false`.

Required fields:

- Source repository or local source identifier: see source-backed Toko evidence
- Source commit SHA: see source-backed Toko evidence
- Source file path: see source-backed Toko evidence
- Mode: see source-backed Toko evidence
- Candid signature: see source-backed Toko evidence
- Request DTO shape: see source-backed Toko evidence
- Response DTO shape: see source-backed Toko evidence
- Malformed input behavior: see source-backed Toko evidence
- Unauthorized behavior: see source-backed Toko evidence
- Batch ordering semantics: see source-backed Toko evidence
- Duplicate-input semantics: see source-backed Toko evidence
- Absent-hash behavior: see source-backed Toko evidence
- Maximum batch size: TBD
- Production-vs-local differences: TBD

### `_immutableObjectStorageBlobsToDelete`

Status: **Source identified**

Owning release: 0.69

Emission in 0.69: yes

Source-backed Toko evidence:

- Source repository or local source identifier: sibling checkout `../toko`
- Source commit SHA: `3ef01afc5f5eeefdb9471f3e010b6562d758c111`
- Source file path:
  `fleets/toko/project/instance/src/ops/immutable_storage.rs`
- Endpoint file path: `fleets/toko/project/instance/src/lib.rs`
- Generated Candid source path:
  `fleets/toko/project/instance/project_instance.did`
- Mode: query
- Candid signature: `() -> (vec text) query`
- Request DTO shape: unit.
- Response DTO shape: `Vec<String>` of pending `sha256:<64-hex>` root hashes.
- Unauthorized behavior: callers not present in the stored gateway-principal
  set receive an empty vector, not a trap or typed error.
- Result ordering: Toko source returns database `all()` order; no stable
  external ordering guarantee has been identified.
- Repeat-return behavior until confirmation: pending hashes remain returned
  until `_immutableObjectStorageConfirmBlobDeletion` clears them.
- Empty pending-deletion behavior: empty vector.

Required fields:

- Source repository or local source identifier: see source-backed Toko evidence
- Source commit SHA: see source-backed Toko evidence
- Source file path: see source-backed Toko evidence
- Mode: see source-backed Toko evidence
- Candid signature: see source-backed Toko evidence
- Request DTO shape: see source-backed Toko evidence
- Response DTO shape: see source-backed Toko evidence
- Unauthorized behavior: see source-backed Toko evidence
- Result ordering: see source-backed Toko evidence
- Maximum batch size: TBD
- Repeat-return behavior until confirmation: see source-backed Toko evidence
- Empty pending-deletion behavior: see source-backed Toko evidence
- Production-vs-local differences: TBD

### `_immutableObjectStorageConfirmBlobDeletion`

Status: **Source identified**

Owning release: 0.69

Emission in 0.69: yes

Source-backed Toko evidence:

- Source repository or local source identifier: sibling checkout `../toko`
- Source commit SHA: `3ef01afc5f5eeefdb9471f3e010b6562d758c111`
- Source file path:
  `fleets/toko/project/instance/src/ops/immutable_storage.rs`
- Endpoint file path: `fleets/toko/project/instance/src/lib.rs`
- Generated Candid source path:
  `fleets/toko/project/instance/project_instance.did`
- Mode: update
- Candid signature: `(vec blob) -> ()`
- Request DTO shape: `Vec<Vec<u8>>`; each entry is expected to be a 32-byte
  root hash and is converted to `sha256:<64-hex>`.
- Response DTO shape: unit.
- Unauthorized behavior: callers not present in the stored gateway-principal
  set are treated as a no-op and receive unit.
- Unknown blob behavior: no-op.
- Live-but-not-pending behavior: the inspected Toko source deletes the stored
  blob row even if no pending-deletion row exists.
- Already-confirmed behavior: no-op.
- Idempotency semantics: repeated calls after deletion are no-ops.

Required fields:

- Source repository or local source identifier: see source-backed Toko evidence
- Source commit SHA: see source-backed Toko evidence
- Source file path: see source-backed Toko evidence
- Mode: see source-backed Toko evidence
- Candid signature: see source-backed Toko evidence
- Request DTO shape: see source-backed Toko evidence
- Response DTO shape: see source-backed Toko evidence
- Unauthorized behavior: see source-backed Toko evidence
- Unknown blob behavior: see source-backed Toko evidence
- Live-but-not-pending behavior: see source-backed Toko evidence
- Already-confirmed behavior: see source-backed Toko evidence
- Idempotency semantics: see source-backed Toko evidence
- Production-vs-local differences: TBD

### `_immutableObjectStorageCreateCertificate`

Status: **Source identified**

Owning release: 0.69

Emission in 0.69: yes

Source-backed Toko evidence:

- Source repository or local source identifier: sibling checkout `../toko`
- Source commit SHA: `3ef01afc5f5eeefdb9471f3e010b6562d758c111`
- Source file path:
  `fleets/toko/project/instance/src/ops/immutable_storage.rs`
- Endpoint file path: `fleets/toko/project/instance/src/lib.rs`
- Generated Candid source path:
  `fleets/toko/project/instance/project_instance.did`
- Mode: update
- Candid signature: `(text) -> (CreateCertificateResult)`
- Request DTO shape: `String` containing `sha256:<64-hex>`.
- Response DTO shape:
  `CreateCertificateResult { method : text; blob_hash : text }`.
- Certificate material source: the Candid response does not contain the upload
  certificate; the frontend extracts the IC update response certificate from
  the agent call and submits that to the gateway as `OwnerEgressSignature`.
- Mutation-before-certificate behavior: the inspected source registers the
  stored blob before returning the Candid response.
- Rollback or no-rollback behavior: no explicit rollback behavior was found;
  endpoint errors trap before a successful return.
- Repeated create behavior: idempotent when the blob root is already stored.
- Metadata conflict/enrichment behavior: no metadata enrichment or conflict
  behavior was found in the inspected source.
- Unauthorized behavior: caller must have Toko `AssetsManage`; failures trap.
- Malformed request behavior: non-`sha256:<64-hex>` input fails validation and
  traps.

Required fields:

- Source repository or local source identifier: see source-backed Toko evidence
- Source commit SHA: see source-backed Toko evidence
- Source file path: see source-backed Toko evidence
- Mode: see source-backed Toko evidence
- Candid signature: see source-backed Toko evidence
- Request DTO shape: see source-backed Toko evidence
- Response DTO shape: see source-backed Toko evidence
- Certificate material source: see source-backed Toko evidence
- Mutation-before-certificate behavior: see source-backed Toko evidence
- Rollback or no-rollback behavior: see source-backed Toko evidence
- Repeated create behavior: see source-backed Toko evidence
- Metadata conflict/enrichment behavior: see source-backed Toko evidence
- Unauthorized behavior: see source-backed Toko evidence
- Malformed request behavior: see source-backed Toko evidence
- Production-vs-local differences: TBD

### `_immutableObjectStorageUpdateGatewayPrincipals`

Status: **Source identified**

Owning release: 0.70

Emission in 0.69: no

Source-backed Toko evidence:

- Source repository or local source identifier: sibling checkout `../toko`
- Source commit SHA: `3ef01afc5f5eeefdb9471f3e010b6562d758c111`
- Source file path:
  `fleets/toko/project/instance/src/ops/immutable_storage.rs`
- Endpoint file path: `fleets/toko/project/instance/src/lib.rs`
- Generated Candid source path:
  `fleets/toko/project/instance/project_instance.did`
- Mode: update
- Candid signature: `() -> ()`
- Request DTO shape: unit.
- Response DTO shape: unit.
- Unauthorized behavior: no caller guard was found on the inspected Toko
  endpoint; the endpoint traps only if Cashier sync or local storage mutation
  fails.
- Cashier dependency: calls `storage_gateway_principal_list_v1`, decodes a
  `Vec<Principal>`, deletes all existing gateway principals, and inserts the
  returned set.

Required fields:

- Source repository or local source identifier: see source-backed Toko evidence
- Source commit SHA: see source-backed Toko evidence
- Source file path: see source-backed Toko evidence
- Mode: see source-backed Toko evidence
- Candid signature: see source-backed Toko evidence
- Request DTO shape: see source-backed Toko evidence
- Response DTO shape: see source-backed Toko evidence
- Unauthorized behavior: see source-backed Toko evidence
- Cashier dependency: see source-backed Toko evidence
- Production-vs-local differences: TBD

### `_immutableObjectStorageFundFromProjectCycles`

Status: **Source identified**

Owning release: 0.70

Emission in 0.69: no

Source-backed Toko evidence:

- Source repository or local source identifier: sibling checkout `../toko`
- Source commit SHA: `3ef01afc5f5eeefdb9471f3e010b6562d758c111`
- Source file path:
  `fleets/toko/project/instance/src/ops/immutable_storage.rs`
- Endpoint file path: `fleets/toko/project/instance/src/lib.rs`
- Generated Candid source path:
  `fleets/toko/project/instance/project_instance.did`
- Mode: update
- Candid signature: `(nat) -> (BlobProjectCyclesTopUpReport)`
- Request DTO shape: `u128` / Candid `nat` requested cycles.
- Response DTO shape:
  `BlobProjectCyclesTopUpReport { requested_cycles : nat; attached_cycles : nat; project_cycles_before : nat; project_cycles_after : nat; reserve_cycles : nat; cashier_total_after : nat; skipped_reason : opt text }`.
- Cycle attachment requirements: caller does not attach cycles to this endpoint;
  the project canister attaches up to the requested amount to Cashier
  `account_top_up_v1` while preserving a 2T project-cycle reserve.
- Unauthorized behavior: caller must be the parent canister; failures trap.
- Funding success/failure behavior: zero transferable cycles returns a skipped
  report; Cashier success returns updated balances; Cashier or decode failures
  trap through the endpoint wrapper.

Required fields:

- Source repository or local source identifier: see source-backed Toko evidence
- Source commit SHA: see source-backed Toko evidence
- Source file path: see source-backed Toko evidence
- Mode: see source-backed Toko evidence
- Candid signature: see source-backed Toko evidence
- Request DTO shape: see source-backed Toko evidence
- Response DTO shape: see source-backed Toko evidence
- Cycle attachment requirements: see source-backed Toko evidence
- Unauthorized behavior: see source-backed Toko evidence
- Funding success/failure behavior: see source-backed Toko evidence
- Production-vs-local differences: TBD

## Compatibility Notes

### Toko

Status: **Incomplete - development source inspected, migration strategy unresolved**

Source evidence captured:

- Local source identifier: sibling checkout `../toko`
- Checked-out `boss` `HEAD` commit:
  `97aafee9eeb73ae0517f9788df688bb96ae0a9ff`
- Checked-out `boss` `HEAD` note: exact `git grep` found no
  `_immutableObjectStorage*`, `BlobRootHash`, gateway status, or Cashier method
  surface in the committed files. The worktree is user-managed and later showed
  unmerged pull state, so inspect development evidence by commit rather than
  by working tree contents.
- Development evidence commit:
  `3ef01afc5f5eeefdb9471f3e010b6562d758c111` on `origin/development`.
- Development inspection note: inspected locally with `git show` / `git grep`
  without checking out or mutating the Toko worktree.
- Legacy asset/chunk compatibility evidence commit:
  `600dcfbe91c30311c5896f3ac0399d27e2e36ab6`.

Development Toko blob-storage source state:

- `backend/src/design/src/app/asset/blob.rs` defines `BlobRootHash` as
  `sha256:<64-hex>` text.
- `backend/src/design/src/entity/project/instance/storage.rs` adds
  `StoredBlob.root_hash`, `BlobDeletionPending.root_hash`,
  `StorageGatewayPrincipal.gateway_principal`, and
  `ProjectStorageConfig.cashier_canister_id`.
- `fleets/toko/project/instance/src/ops/immutable_storage.rs` validates
  `sha256:<64-hex>` root strings, registers stored blobs idempotently, marks
  pending deletion rows, exposes gateway-only deletion queues, and confirms
  deletion from gateway hash bytes.
- `frontend/src/lib/storage/storage-client.ts` computes the blob root from the
  Caffeine gateway hash tree and calls
  `_immutableObjectStorageCreateCertificate` with the resulting
  `sha256:<64-hex>` string.

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

- Development Toko live blob state uses `StoredBlob.root_hash` with the
  Caffeine `sha256:<64-hex>` root string and therefore matches Canic's planned
  `BlobRootHash` shape for newly registered blobs.
- Existing legacy Toko live asset state is chunk-upload state, not the
  Caffeine immutable object-storage gateway protocol.
- Existing legacy Toko pending deletion state shape: none found.
- Existing legacy Toko gateway-principal state shape: none found.
- Mapping from legacy Toko asset identity into Canic `BlobRootHash`: unresolved.
  Legacy Toko has asset ULIDs, text references, optional unset file metadata
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
