# Blob Storage Cashier Protocol Inventory

Status: **Complete**

Release line: 0.70

Last updated: 2026-06-20

## Purpose

This inventory is the source-of-truth gate for Canic's 0.70 blob-storage
Cashier integration.

No `blob-storage-billing` feature, Cashier DTO, Cashier client wrapper, billing
stable record, gateway-principal sync workflow, funding workflow, billing
endpoint macro, billing Candid snapshot, or billing behavior test may merge
unless this inventory remains complete and cites exact protocol sources.

This inventory does not replace the 0.69 gateway protocol inventory. The 0.70
billing line required both inventories to be complete before implementation:

- `docs/contracts/BLOB_STORAGE_INVENTORY.md`
- `docs/contracts/BLOB_STORAGE_CASHIER_INVENTORY.md`

## Current Finding

The actual Cashier canister implementation source has not been identified in
this repository. For the 0.70 backend MVP, the maintainer has approved current
Toko `boss` as the protocol source for the Cashier methods used by blob-storage
billing.

Initial exact-match searches in the local tree found only Canic design notes
for:

- `account_balance_get_v1`
- `account_top_up_v1`
- `storage_gateway_principal_list_v1`

The local Toko `boss` commit
`9ca150b396a2bde42f2b8977a04a7ca2c6172b56` now provides inspected
consumer/wrapper evidence for the Cashier methods used by blob-storage status,
gateway-principal sync, and funding. That evidence records the accepted 0.70
MVP Candid shapes, DTOs, and call flow. It is not Cashier implementation source;
future Cashier source or deployed Candid may supersede this inventory if it
contradicts the Toko-backed contract.

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

Historical inventory effect at the time:

- These results were useful candidate Toko consumer/wrapper evidence.
- They were not yet accepted as authoritative Cashier protocol source at this
  stage.
- The later local Toko `boss` inspection and maintainer approval supersede this
  blocked finding.

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

Historical inventory effect at the time:

- This was enough to describe Toko's Cashier wrapper expectations.
- It was not accepted as an implementation unlock at this stage.
- The later local Toko `boss` inspection and maintainer approval supersede this
  blocked finding.

### 2026-06-20 Local Toko Boss Commit Inspection

Search scope:

- `/home/adam/projects/toko`

Checkout state:

- Checked-out branch: `boss`
- Checked-out `HEAD` commit:
  `9ca150b396a2bde42f2b8977a04a7ca2c6172b56`
- Worktree state: clean at inspection time.

Exact source search:

```text
git grep -n "account_balance_get_v1\|account_top_up_v1\|storage_gateway_principal_list_v1\|get_blob_storage_status" HEAD -- .
git grep -n "immutable_storage\|blob_storage" HEAD -- .
find . -path './target' -prune -o -name '*.did' -print
```

Result:

- Current `boss` contains Toko project-hub wrapper calls to
  `account_balance_get_v1` at
  `fleets/toko/project/hub/src/ops/blob_storage.rs`.
- Current `boss` contains Toko project-instance wrapper calls to
  `storage_gateway_principal_list_v1` and `account_top_up_v1` at
  `fleets/toko/project/instance/src/ops/immutable_storage.rs`.
- Current `boss` contains generated project-hub and project-instance Candid at:
  - `fleets/toko/project/hub/project_hub.did`
  - `fleets/toko/project/instance/project_instance.did`

Inventory effect:

- The stale local-`boss` no-source finding is superseded.
- Maintainer-approved Toko wrapper evidence is accepted as the 0.70 MVP
  protocol source for the three Cashier methods used by blob-storage billing.
- Actual Cashier implementation source or deployed Cashier Candid remains
  useful follow-up evidence, but is not required to begin the Toko-backed
  0.70 MVP.

## Completion Criteria

This inventory is complete only when every required field below is filled from
an upstream source, generated Candid artifact, deployed interface, or other
maintainer-approved protocol source. For 0.70 MVP, current Toko `boss` is the
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

- `Missing source`: no immutable or maintainer-approved protocol source has
  been identified.
- `Source identified`: source repository or local source identifier, immutable
  provenance, and per-method source path are recorded, but Candid or behavior
  fields remain incomplete.
- `Snapshot captured`: source metadata and exact Candid are recorded, but
  behavior fields or wrapper interoperability notes remain incomplete.
- `Complete`: every required source, Candid, behavior, and interoperability field
  is filled from cited protocol evidence.

Design-note statements may describe expected ownership or implementation
direction, but they do not satisfy source, Candid, DTO, behavior, or
interoperability fields. Keep unknown protocol facts unresolved instead of
inferring them from the 0.70 design.

## Method Inventory

Every method section must keep design-only facts separate from source-backed or
maintainer-approved facts. Do not move a method out of `Missing source` until
at least the source identifier, immutable provenance, per-method source path,
and generated, deployed, or maintainer-approved protocol evidence are recorded.

### `account_balance_get_v1`

Status: **Complete**

Owning release: 0.70

Accepted protocol source:

- Source repository or local source identifier: sibling checkout
  `/home/adam/projects/toko`
- Source commit SHA:
  `9ca150b396a2bde42f2b8977a04a7ca2c6172b56`
- Source file path: `fleets/toko/project/hub/src/ops/blob_storage.rs`
- Source file commit SHA:
  `9ca150b396a2bde42f2b8977a04a7ca2c6172b56`
- Production Cashier canister ID: `72ch2-fiaaa-aaaar-qbsvq-cai`
- Generated Candid source path or command used to generate the Candid:
  maintainer-approved Toko `CandidType` call-site DTOs in the source path
  above.

Mode and Candid:

- Mode: update, called directly through `ic_cdk::call::Call::bounded_wait`.
- Candid signature:
  ```did
  account_balance_get_v1 : (
    record { account : principal },
  ) -> (
    variant {
      Ok : record {
        account_cycle_balances : AccountCycleBalances;
        account : principal;
      };
      Err : AccountBalanceGetError;
    },
  );
  ```
- Shared nested record:
  ```did
  type AccountCycleBalances = record {
    total : int;
    cycles_prepaid : int;
    cycles_promo : int;
    debt_target : DebtTarget;
    cycles_ledger : int;
  };
  type DebtTarget = variant { Prepaid; Ledger };
  ```
- Error variant:
  ```did
  type AccountBalanceGetError = variant {
    AccountNotFound;
    InternalError : text;
  };
  ```

Behavior:

- Request DTO shape: record with `account : principal`.
- Response DTO shape: `Ok` returns `account_cycle_balances` and `account`;
  `Err` returns `AccountNotFound` or `InternalError(text)`.
- Balance units: cycles.
- Integer width: Cashier returns signed Candid `int`; Canic wrappers convert to
  unsigned `u128`/`nat` for public status and reject negative or too-large
  balances as malformed Cashier responses.
- Result/error variant behavior: `AccountNotFound` maps to zero prepaid and
  zero usable balance for project upload readiness; `InternalError(text)` maps
  to a Cashier balance-read failure.
- Trap/reject behavior: inter-canister call rejection, trap, timeout, or
  transport failure maps to a Cashier balance-read failure.
- Malformed request behavior: Canic produces only the typed request record; no
  public Canic path accepts arbitrary Cashier request bytes.
- Malformed response behavior expected from Canic wrappers: Candid decode
  failure, unknown variants, missing fields, negative cycle balances, and
  balances outside `u128` map to typed Cashier decode/unexpected-response
  errors.
- Production-vs-local differences: production defaults to
  `72ch2-fiaaa-aaaar-qbsvq-cai` only when explicitly configured; tests must use
  injected mock Cashier principals and must not call production Cashier.

### `account_top_up_v1`

Status: **Complete**

Owning release: 0.70

Accepted protocol source:

- Source repository or local source identifier: sibling checkout
  `/home/adam/projects/toko`
- Source commit SHA:
  `9ca150b396a2bde42f2b8977a04a7ca2c6172b56`
- Source file path:
  `fleets/toko/project/instance/src/ops/immutable_storage.rs`
- Source file commit SHA:
  `9ca150b396a2bde42f2b8977a04a7ca2c6172b56`
- Production Cashier canister ID: `72ch2-fiaaa-aaaar-qbsvq-cai`
- Generated Candid source path or command used to generate the Candid:
  maintainer-approved Toko `CandidType` call-site DTOs in the source path
  above.

Mode and Candid:

- Mode: update, called directly through `ic_cdk::call::Call::bounded_wait`.
- Candid signature:
  ```did
  account_top_up_v1 : (
    opt record {
      target_balance : opt nat;
      account : opt principal;
    },
  ) -> (
    variant {
      Ok : record {
        balance : AccountCycleBalances;
        message : text;
      };
      Err : AccountTopUpError;
    },
  );
  ```
- Shared nested record:
  ```did
  type AccountCycleBalances = record {
    total : int;
    cycles_prepaid : int;
    cycles_promo : int;
    debt_target : DebtTarget;
    cycles_ledger : int;
  };
  type DebtTarget = variant { Prepaid; Ledger };
  ```
- Error variant:
  ```did
  type AccountTopUpError = variant {
    NotAuthorized : principal;
    AccountBalanceOverflow;
    InternalError : text;
    TopUpWithoutCycles;
  };
  ```

Behavior:

- Request DTO shape: optional record with `target_balance : opt nat` and
  `account : opt principal`. Toko passes `Some(record { target_balance = None;
  account = Some(project_pid) })` for project-as-payment-account funding.
- Response DTO shape: `Ok` returns the resulting `balance` and a human-readable
  `message`; `Err` returns one of the inventoried `AccountTopUpError` variants.
- Cycle attachment requirements: caller attaches the exact top-up amount as
  call cycles. Canic must not call this method without attached cycles when a
  top-up is intended.
- Balance mutation timing: success is represented by `Ok`; Canic treats the
  returned `balance.total` as the post-top-up Cashier total observed by the
  call.
- Result/error variant behavior: all `Err` variants map to typed top-up failure
  paths. `TopUpWithoutCycles` is the expected Cashier-side failure when cycles
  are not attached.
- Trap/reject behavior: inter-canister call rejection, trap, timeout, or
  transport failure maps to a typed top-up call failure and releases any local
  funding single-flight guard.
- Malformed request behavior: Canic produces only the typed optional request
  record; no public Canic path accepts arbitrary Cashier request bytes.
- Malformed response behavior expected from Canic wrappers: Candid decode
  failure, unknown variants, missing fields, negative balances, and balances
  outside `u128` map to typed Cashier decode/unexpected-response errors.
- Funding success/failure behavior: Canic reports success only after `Ok` is
  decoded and the returned total balance is representable; all errors preserve
  project reserve accounting and release the transient funding guard.
- Production-vs-local differences: production defaults to
  `72ch2-fiaaa-aaaar-qbsvq-cai` only when explicitly configured; tests must use
  injected mock Cashier principals and must not call production Cashier.

### `storage_gateway_principal_list_v1`

Status: **Complete**

Owning release: 0.70

Accepted protocol source:

- Source repository or local source identifier: sibling checkout
  `/home/adam/projects/toko`
- Source commit SHA:
  `9ca150b396a2bde42f2b8977a04a7ca2c6172b56`
- Source file path:
  `fleets/toko/project/instance/src/ops/immutable_storage.rs`
- Source file commit SHA:
  `9ca150b396a2bde42f2b8977a04a7ca2c6172b56`
- Production Cashier canister ID: `72ch2-fiaaa-aaaar-qbsvq-cai`
- Generated Candid source path or command used to generate the Candid:
  maintainer-approved Toko `CandidType` call-site expectation in the source path
  above.

Mode and Candid:

- Mode: update, called directly through `ic_cdk::call::Call::bounded_wait`.
- Candid signature:
  ```did
  storage_gateway_principal_list_v1 : () -> (vec principal);
  ```

Behavior:

- Request DTO shape: no arguments.
- Response DTO shape: `vec principal`.
- Empty-list behavior: malformed; preserves the previous gateway-principal set.
  Toko's call-site type allows an empty vector, but Canic sync treats empty
  successful responses as malformed Cashier responses. This prevents transient
  or misconfigured Cashier responses from atomically wiping all upload gateway
  authorization.
- Duplicate-principal behavior: duplicates have no additional meaning. Canic
  deduplicates before writing gateway-principal state.
- Anonymous-principal behavior: Canic rejects anonymous principals as malformed
  Cashier responses.
- Management-canister-principal behavior: Canic rejects the management canister
  principal as a malformed Cashier response.
- Maximum principal count: Cashier does not expose a Toko-side limit. Canic
  must enforce a local maximum before stable writes to bound memory and response
  costs.
- Result/error variants: no result wrapper in the accepted Candid; success is a
  decoded vector, while call failure or malformed response maps to a typed sync
  failure.
- Trap/reject behavior: inter-canister call rejection, trap, timeout, or
  transport failure preserves the previous gateway-principal set and reports a
  sync failure.
- Malformed request behavior: Canic sends no arguments; no public Canic path
  accepts arbitrary Cashier request bytes.
- Malformed response behavior expected from Canic wrappers: Candid decode
  failure, empty lists, invalid principals, and responses exceeding the
  configured maximum preserve the previous gateway-principal set and report a
  typed sync failure.
- Production-vs-local differences: production defaults to
  `72ch2-fiaaa-aaaar-qbsvq-cai` only when explicitly configured; tests must use
  injected mock Cashier principals and must not call production Cashier.

## Optional Cashier Methods

Status: **Complete**

Accepted MVP scope:

- Toko `boss` exposes exactly three Cashier method call sites needed by
  blob-storage billing:
  `account_balance_get_v1`, `account_top_up_v1`, and
  `storage_gateway_principal_list_v1`.
- No additional Cashier method is required for 0.70 MVP balance reads,
  project-as-payment-account top-up, or gateway-principal sync.
- Toko project wrapper endpoints such as `get_blob_storage_status`,
  `_immutableObjectStorageUpdateGatewayPrincipals`, and
  `_immutableObjectStorageFundFromProjectCycles` are not Cashier methods. They
  remain Canic/project-facing surfaces whose gateway Candid is sourced from the
  0.69 gateway inventory.
- Future Cashier source or deployed Candid may reveal additional Cashier
  methods. Those methods are deferred unless they are required to preserve the
  three accepted Toko-backed 0.70 flows.

## Implementation Gate

The following actions were blocked while this document was incomplete and are
now unblocked for the Toko-backed 0.70 MVP:

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
`scripts/ci/check-blob-storage-cashier-inventory-gate.sh`. While the status was
incomplete, the guard rejected blob-storage billing feature metadata,
source/module paths, Cashier method literals, billing status endpoint literals,
and public Cashier/billing API/model names outside this protocol inventory and
design documentation. Now that this inventory is marked `Complete`, the same
guard verifies that all required method sections are present and individually
complete, have no unresolved fields, and that the optional Cashier methods
section is also complete.

Implementation state:

- The 0.70 backend MVP consumes this inventory through checked-in Candid
  snapshots, typed Cashier DTOs/wrappers, explicit billing config, mock Cashier
  tests, gateway-principal sync, project-cycle funding, and read-only backend
  status.
- Production Cashier stays disabled in tests and must be injected by explicit
  configuration.
- Update this inventory and the 0.70 design if actual Cashier source or
  deployed Candid later contradicts the Toko-backed MVP contract.
