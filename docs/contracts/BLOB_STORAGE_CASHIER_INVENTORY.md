# Blob Storage Cashier Protocol Inventory

Status: **Incomplete - implementation blocked**

Release line: 0.70

Last updated: 2026-06-19

## Purpose

This inventory is the source-of-truth gate for Canic's 0.70 blob-storage
Cashier integration.

No `blob-storage-billing` feature, Cashier DTO, Cashier client wrapper, billing
stable record, gateway-principal sync workflow, funding workflow, billing
endpoint macro, billing Candid snapshot, or billing behavior test may merge
until this inventory is complete and cites exact protocol sources.

This inventory does not replace the 0.69 gateway protocol inventory. The 0.70
billing line remains blocked until both inventories are complete:

- `docs/contracts/BLOB_STORAGE_INVENTORY.md`
- `docs/contracts/BLOB_STORAGE_CASHIER_INVENTORY.md`

## Current Finding

The Cashier protocol source has not yet been identified in this repository.

Initial exact-match searches in the local tree found only Canic design notes
for:

- `account_balance_get_v1`
- `account_top_up_v1`
- `storage_gateway_principal_list_v1`

No Candid signature, source repository URL, source commit SHA, deployed `.did`,
or upstream implementation file has been recorded yet.

The local Toko `origin/development` commit
`3ef01afc5f5eeefdb9471f3e010b6562d758c111` now provides inspected
consumer/wrapper evidence for the Cashier methods used by blob-storage status
and funding. That evidence records Toko's expected DTOs and call flow, but it
is not the Cashier implementation source and does not unlock billing work.

## Protocol Source Search Log

### 2026-06-19 GitHub Installed Repository Code Search

Search scope:

- GitHub App installed repositories visible to this session:
  `dragginzgame/canic` and `dragginzgame/toko`

Search terms:

```text
account_balance_get_v1
account_top_up_v1
storage_gateway_principal_list_v1
get_blob_storage_status
blob_storage
```

Result:

- No separate Cashier source repository was visible through the installed
  GitHub repositories.
- GitHub code search found newer Toko consumer-side candidates at commit
  `3ef01afc5f5eeefdb9471f3e010b6562d758c111`.
- `account_balance_get_v1` matched:
  - `fleets/toko/project/hub/src/ops/blob_storage.rs`
  - `backend/src/canisters/project/hub/src/ops/blob_storage.rs`
- `get_blob_storage_status` matched:
  - `fleets/toko/project/hub/src/lib.rs`
  - `fleets/toko/project/hub/src/ops/blob_storage.rs`
  - `backend/src/canisters/project/hub/src/ops/blob_storage.rs`
  - `fleets/toko/project/hub/project_hub.did`
  - `frontend/src/generated/declarations/project_hub/project_hub.did.js`
  - `frontend/src/generated/declarations/project_hub/project_hub.did.d.ts`
- `account_top_up_v1` and `storage_gateway_principal_list_v1` matched:
  - `fleets/toko/project/instance/src/ops/immutable_storage.rs`

Local inspection limit, superseded by the later local Toko inspection below:

- The local Toko checkout is at
  `600dcfbe91c30311c5896f3ac0399d27e2e36ab6` on branch `boss`, with local
  Cargo metadata edits, and does not contain GitHub-indexed commit
  `3ef01afc5f5eeefdb9471f3e010b6562d758c111`.
- The local `gh` token is invalid in this session, so the indexed private Toko
  files could not be fetched through `gh`.
- The dirty local Toko checkout was not fetched or modified.

Inventory effect:

- These results are useful candidate Toko consumer/wrapper evidence.
- They are not authoritative Cashier protocol source and do not satisfy
  source-backed Candid, DTO, or behavior fields.
- Every Cashier method remains `Missing source`.
- The billing implementation gate remains closed.

Superseded next-step note:

- The Toko candidate commit was later inspected locally; see
  `2026-06-19 Local Toko Development Commit Inspection`.
- Locate the actual Cashier implementation or generated/deployed Cashier
  `.did`.
- Continue to keep Toko consumer/wrapper evidence separate from Cashier
  protocol source evidence.

### 2026-06-19 Local Toko Development Commit Inspection

Search scope:

- `/home/adam/projects/toko`

Checkout state:

- Checked-out branch: `boss`
- Checked-out `HEAD` commit:
  `97aafee9eeb73ae0517f9788df688bb96ae0a9ff`
- Worktree note: the Toko checkout is user-managed and later showed unmerged
  pull state; do not treat worktree conflict contents as canonical evidence.
- Exact `git grep` on checked-out `HEAD` for `account_balance_get_v1`,
  `account_top_up_v1`, `storage_gateway_principal_list_v1`,
  `get_blob_storage_status`, and `Cashier` returned no protocol matches.
- Candidate commit
  `3ef01afc5f5eeefdb9471f3e010b6562d758c111` exists locally on
  `origin/development` and was inspected with `git show` / `git grep` without
  checking it out.

Source-backed Toko consumer/wrapper evidence at commit
`3ef01afc5f5eeefdb9471f3e010b6562d758c111`:

- `fleets/toko/project/hub/src/ops/blob_storage.rs` calls
  `account_balance_get_v1` and exposes project-level
  `BlobStorageBillingStatus`.
- `fleets/toko/project/hub/project_hub.did` exposes
  `get_blob_storage_status : (DelegatedToken, principal) -> (Result_11)`,
  where `Result_11` is `Ok : BlobStorageBillingStatus` or `Err : Error`.
- `fleets/toko/project/instance/src/ops/immutable_storage.rs` calls
  `storage_gateway_principal_list_v1` and `account_top_up_v1`.
- `fleets/toko/project/instance/project_instance.did` exposes
  `_immutableObjectStorageUpdateGatewayPrincipals` and
  `_immutableObjectStorageFundFromProjectCycles`, but those are project
  canister wrapper endpoints, not Cashier endpoints.

Toko call-site DTO evidence:

- `account_balance_get_v1` request: `{ account : principal }`.
- `account_balance_get_v1` expected result:
  `Ok { account_cycle_balances; account }` or
  `Err { AccountNotFound | InternalError(text) }`.
- `account_top_up_v1` request:
  `{ target_balance : opt nat; account : opt principal }` with cycles attached
  by the caller.
- `account_top_up_v1` expected result:
  `Ok { balance; message : text }` or
  `Err { NotAuthorized(principal) | AccountBalanceOverflow | InternalError(text) | TopUpWithoutCycles }`.
- Shared `AccountCycleBalances` fields expected by Toko:
  `total`, `cycles_prepaid`, `cycles_promo`, `debt_target`, and
  `cycles_ledger`.
- `storage_gateway_principal_list_v1` expected response:
  `Vec<Principal>`.

Inventory effect:

- This is enough to describe Toko's Cashier wrapper expectations.
- It is not enough to mark any Cashier method `Source identified`, because the
  actual Cashier implementation or generated/deployed Cashier `.did` is still
  missing.
- The billing implementation gate remains closed.

## Completion Criteria

This inventory is complete only when every required field below is filled from
an upstream source, generated Candid artifact, deployed interface, or other
maintainer-approved protocol source.

Required source metadata:

- Production Cashier canister ID.
- Source repository URL or local source identifier.
- Source commit SHA or immutable provenance identifier.
- Per-method source file path.
- Per-method source file commit SHA when different from the repository SHA.
- Generated Candid source path or command used to generate the Candid.

Required behavior metadata:

- Method name.
- Query/update mode.
- Exact Candid signature.
- Request DTO shape.
- Response DTO shape.
- Nested records and variants.
- Result/error variant behavior.
- Trap, reject, and result behavior.
- Malformed request behavior.
- Malformed response behavior expected from Canic wrappers.
- Cycle-attachment requirements.
- Balance units and integer width.
- Empty gateway-principal list behavior.
- Duplicate gateway-principal behavior.
- Optional Cashier methods and why each is implemented or deferred.
- Production-vs-local behavior differences.

### Status Vocabulary

Method status values are intentionally narrow:

- `Missing source`: no immutable protocol source has been identified.
- `Source identified`: source repository or local source identifier, immutable
  provenance, and per-method source path are recorded, but Candid or behavior
  fields remain incomplete.
- `Snapshot captured`: source metadata and exact Candid are recorded, but
  behavior fields or wrapper compatibility notes remain incomplete.
- `Complete`: every required source, Candid, behavior, and compatibility field
  is filled from cited protocol evidence.

Design-note statements may describe expected ownership or implementation
direction, but they do not satisfy source, Candid, DTO, behavior, or
compatibility fields. Keep unknown protocol facts as `TBD` instead of inferring
them from the 0.70 design.

## Method Inventory

Every method section must keep design-only facts separate from source-backed
facts. Do not move a method out of `Missing source` until at least the source
identifier, immutable provenance, per-method source path, and generated or
deployed Candid source are recorded.

### `account_balance_get_v1`

Status: **Missing source**

Owning release: 0.70

Known from design only:

- Reads a Cashier account balance for blob-storage readiness/status.
- Canic wrappers must not invent balance units, integer width, or result/error
  variants.

Toko call-site evidence only:

- Toko source commit:
  `3ef01afc5f5eeefdb9471f3e010b6562d758c111`
- Toko wrapper path: `fleets/toko/project/hub/src/ops/blob_storage.rs`
- Toko request shape: `{ account : principal }`.
- Toko expected response shape:
  `Ok { account_cycle_balances; account }` or
  `Err { AccountNotFound | InternalError(text) }`.
- Toko expected balance record fields:
  `total`, `cycles_prepaid`, `cycles_promo`, `debt_target`, and
  `cycles_ledger`.
- This is not Cashier implementation or generated Cashier Candid evidence.

Required fields:

- Source repository or local source identifier: TBD
- Source commit SHA: TBD
- Source file path: TBD
- Mode: TBD
- Candid signature: TBD
- Request DTO shape: TBD
- Response DTO shape: TBD
- Balance units: TBD
- Integer width: TBD
- Result/error variants: TBD
- Trap/reject behavior: TBD
- Malformed request behavior: TBD
- Malformed response behavior expected from Canic wrappers: TBD
- Production-vs-local differences: TBD

### `account_top_up_v1`

Status: **Missing source**

Owning release: 0.70

Known from design only:

- Receives attached cycles from the project-as-payment-account funding path.
- Canic funding policy must not attach cycles until exact cycle attachment and
  success/failure behavior is inventoried.

Toko call-site evidence only:

- Toko source commit:
  `3ef01afc5f5eeefdb9471f3e010b6562d758c111`
- Toko wrapper path:
  `fleets/toko/project/instance/src/ops/immutable_storage.rs`
- Toko request shape:
  `{ target_balance : opt nat; account : opt principal }`.
- Toko expected response shape:
  `Ok { balance; message : text }` or
  `Err { NotAuthorized(principal) | AccountBalanceOverflow | InternalError(text) | TopUpWithoutCycles }`.
- Toko attaches project canister cycles to the Cashier call.
- This is not Cashier implementation or generated Cashier Candid evidence.

Required fields:

- Source repository or local source identifier: TBD
- Source commit SHA: TBD
- Source file path: TBD
- Mode: TBD
- Candid signature: TBD
- Request DTO shape: TBD
- Response DTO shape: TBD
- Cycle attachment requirements: TBD
- Balance mutation timing: TBD
- Result/error variants: TBD
- Trap/reject behavior: TBD
- Malformed request behavior: TBD
- Funding success/failure behavior: TBD
- Production-vs-local differences: TBD

### `storage_gateway_principal_list_v1`

Status: **Missing source**

Owning release: 0.70

Known from design only:

- Provides the gateway principals that 0.70 syncs into the 0.69
  gateway-principal store.
- Canic must not invent empty-list, duplicate, anonymous-principal, or
  management-canister-principal semantics.

Toko call-site evidence only:

- Toko source commit:
  `3ef01afc5f5eeefdb9471f3e010b6562d758c111`
- Toko wrapper path:
  `fleets/toko/project/instance/src/ops/immutable_storage.rs`
- Toko expected response shape: `Vec<Principal>`.
- This is not Cashier implementation or generated Cashier Candid evidence.

Required fields:

- Source repository or local source identifier: TBD
- Source commit SHA: TBD
- Source file path: TBD
- Mode: TBD
- Candid signature: TBD
- Request DTO shape: TBD
- Response DTO shape: TBD
- Empty-list behavior: TBD
- Duplicate-principal behavior: TBD
- Anonymous-principal behavior: TBD
- Management-canister-principal behavior: TBD
- Maximum principal count: TBD
- Result/error variants: TBD
- Trap/reject behavior: TBD
- Malformed request behavior: TBD
- Malformed response behavior expected from Canic wrappers: TBD
- Production-vs-local differences: TBD

## Optional Cashier Methods

Status: **Incomplete**

Required before implementation:

- Complete list of Cashier methods discovered in the source/deployed Candid.
- For each omitted method, a reason why 0.70 does not need it.
- Confirmation that no omitted method is required for balance reads,
  project-as-payment-account top-up, or gateway-principal sync.

## Implementation Gate

The following actions are blocked while this document remains incomplete:

- Adding the `blob-storage-billing` feature.
- Adding Cashier DTOs or Candid snapshots.
- Adding Cashier call wrappers.
- Adding billing config stable records.
- Adding gateway-principal sync storage or workflow.
- Adding funding policy or funding workflow.
- Emitting `_immutableObjectStorageUpdateGatewayPrincipals`.
- Emitting `_immutableObjectStorageFundFromProjectCycles`.
- Emitting `get_blob_storage_status`.
- Adding billing macro tests, Cashier wrapper tests, or PocketIC billing
  behavior tests that assert protocol behavior.

This gate is enforced in CI and local Make test/release-bump paths by
`scripts/ci/check-blob-storage-cashier-inventory-gate.sh`. While the status
remains incomplete, the guard rejects blob-storage billing feature metadata,
source/module paths, Cashier method literals, billing status endpoint literals,
and public Cashier/billing API/model names outside this protocol inventory and
design documentation. When this inventory is marked `Complete`, the same guard
verifies that all required method sections are present and individually
complete, have no `TBD` fields, and that the optional Cashier methods section
is also complete.

The only safe next steps are:

- Locate the upstream Cashier source or generated `.did`.
- Fill the method inventory from immutable source references.
- Add Candid snapshots copied or generated from the inventoried source.
- Update the 0.70 design if the protocol source contradicts current design
  assumptions.
