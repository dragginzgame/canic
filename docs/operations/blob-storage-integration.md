# Blob Storage Integration

This runbook documents the 0.69 non-billing blob-storage integration path for
consumer canisters.

It is intentionally narrower than the design document. Use it when wiring a
downstream canister to Canic's current blob-storage backend.

## Scope

0.69 provides the non-billing immutable object-storage gateway surface:

1. `_immutableObjectStorageBlobsAreLive`
2. `_immutableObjectStorageBlobsToDelete`
3. `_immutableObjectStorageConfirmBlobDeletion`
4. `_immutableObjectStorageCreateCertificate`

0.69 does not provide Cashier balance checks, cycle top-up, gateway-principal
sync from Cashier, browser/frontend provisioning, or public blob-storage admin
surfaces. Those remain gated to the later billing line.

## Cargo Wiring

Enable the `blob-storage` feature in the canister crate that emits the gateway
endpoints:

```toml
[dependencies]
canic = { workspace = true, features = ["blob-storage"] }

[features]
blob-storage = ["canic/blob-storage"]
```

The endpoint macro is compiled behind the consuming crate's `blob-storage`
feature. A dedicated blob-enabled canister may include that feature in its
default feature set. Shared crates should keep it opt-in.

Outside the Canic workspace, enable the feature on the canister's existing
`canic` dependency declaration instead of copying a version from this document.

## Endpoint Macro

Emit the four gateway-compatible endpoints from the canister root:

```rust
canic::canic_emit_blob_storage_endpoints!(guard = caller::is_controller());
```

The supplied guard protects only
`_immutableObjectStorageCreateCertificate`. Choose a host-canister policy that
is allowed to register upload certificate requests.

The other three endpoints follow the gateway protocol contract:

- `_immutableObjectStorageBlobsAreLive` is a query and returns `false` for
  malformed 32-byte gateway inputs.
- `_immutableObjectStorageBlobsToDelete` is a query and returns pending
  deletions only to stored gateway principals.
- `_immutableObjectStorageConfirmBlobDeletion` is an update and accepts
  deletion confirmations only from stored gateway principals.

Do not route gateway deletion endpoints through product frontend auth. Gateway
authorization is the stored-principal check.

## Lifecycle API

Consumer backends should use `canic::api::blob_storage::BlobStorageApi` for
internal lifecycle work instead of calling `_immutableObjectStorage*` endpoints.

Use these helpers for the 0.69 lifecycle:

- `create_certificate(root_hash)` registers a live root and returns the
  gateway-compatible certificate DTO.
- `register_live(root_hash, now_ns)` registers a root when certificate creation
  happens outside the endpoint path.
- `require_live(root_hash)` enforces that a consumer record points at a
  registered live blob.
- `is_live(root_hash)` checks whether a root is registered and not pending
  deletion.
- `mark_pending_delete(root_hash, now_ns)` queues a live root for gateway
  deletion.
- `upsert_gateway_principal(principal, now_ns)` and
  `remove_gateway_principal(principal)` maintain the local gateway-principal
  allowlist until the later Cashier sync line exists.
- `stored_blob_count()`, `pending_deletion_count()`, and
  `gateway_principal_count()` return local operational counters that host
  canisters may expose through their own guarded status endpoint.

`upsert_gateway_principal` should be exposed only through an operator,
controller, deployment, or test-only path chosen by the host canister. 0.69 does
not add a production Cashier sync endpoint.

## Root Hash Contract

Canic stores blob roots as canonical `sha256:<64-lowercase-hex>` strings.

Text inputs must use the `sha256:` prefix and exactly 64 hex characters.
Uppercase hex characters are normalized to lowercase. Gateway liveness and
deletion-confirmation endpoint inputs use raw 32-byte root hashes and are
converted to the canonical string internally.

Consumer records should store the same canonical `sha256:<64-hex>` value used
by the backend. Avoid maintaining a second blob identity.

## State Transitions

The 0.69 lifecycle is deliberately small:

1. Absent roots become live through `create_certificate` or `register_live`.
2. Live roots satisfy `require_live` and gateway liveness queries.
3. `mark_pending_delete` moves a live root into pending deletion.
4. Pending roots are no longer live.
5. Stored gateway principals can query pending roots.
6. A stored gateway principal can confirm deletion.
7. Confirmation removes both pending-deletion state and live state.

Repeated live registration, pending-delete marking, and deletion confirmation
are safe. Unknown or already-confirmed deletion confirmations are no-ops.

Stable live roots, pending-deletion rows, and gateway principals survive
canister upgrade.

## Product Flow

Product frontends should not run provisioning, Cashier, or billing flows in
0.69. The application flow should stay on the product backend:

1. Upload or receive a blob root from the storage gateway flow.
2. Store the canonical `sha256:<64-hex>` root on the product record.
3. Before accepting a production asset, call `require_live`.
4. When deleting the product asset, call `mark_pending_delete`.

Local development may use placeholder blob hashes only when the product backend
explicitly allows that for local builds.

## Validation

Useful focused checks for a downstream integration:

```text
cargo check --locked -p <canister> --features blob-storage
cargo clippy --locked -p <canister> --features blob-storage -- -D warnings
cargo test --locked -p canic --test protocol_surface -- --nocapture
cargo test --locked -p canic --features blob-storage --test blob_storage_endpoint_macro -- --nocapture
bash scripts/ci/check-blob-storage-inventory-gate.sh
bash scripts/ci/check-blob-storage-cashier-inventory-gate.sh
```

For Canic's own probe coverage, run:

```text
cargo check --locked -p blob_storage_probe
cargo clippy --locked -p blob_storage_probe -- -D warnings
cargo clippy --locked -p canic-tests --test pic_blob_storage -- -D warnings
POCKET_IC_BIN=<path-to-pocket-ic> cargo test --locked -p canic-tests --test pic_blob_storage -- --nocapture
```

The `blob_storage_probe` test canister exposes controller-only helpers to add
and remove local gateway principals, plus a local count query for integration
assertions. They are for integration testing and downstream wrapper examples
only, not a production Cashier sync path.

## References

- `docs/design/0.69-blob-storage/0.69-design.md`
- `docs/contracts/BLOB_STORAGE_INVENTORY.md`
- `docs/contracts/BLOB_STORAGE_CASHIER_INVENTORY.md`
- `docs/operations/blob-storage-source-handoff.md`
- `crates/canic/src/macros/endpoints/blob_storage.rs`
- `canisters/test/blob_storage_probe/src/lib.rs`
- `crates/canic-tests/tests/pic_blob_storage.rs`
