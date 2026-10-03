# Blob Storage

Blob storage is for application data that is naturally handled as a large piece
of content, such as an uploaded image, document, or media file. It is separate
from snapshots used to back up an entire canister.

Canic provides optional Rust and operator integrations for this storage. The
base feature manages gateway state and administration; a separate billing
feature adds payment status, funding, and readiness checks through Cashier.

## What It Provides

- opt-in `blob-storage` runtime APIs and endpoint macros
- controller-guarded gateway administration
- stable local counters and root-hash state
- opt-in `blob-storage-billing` Cashier integration
- operator status, gateway synchronization, funding, and medic checks
- separate Cargo features so ordinary canisters carry none of this surface

Downstream canisters select the feature explicitly and choose the endpoint
guard appropriate to their application authority.

Remote clients can import all passive request, response and billing value types
from `canic::dto::blob_storage` with `default-features = false` and no blob
features. Only canisters hosting local blob state or workflows should enable
`blob-storage` or `blob-storage-billing`. Importing these DTOs does not select
blob memory allocations, funding workflows or endpoint macros.

This boundary prepares client-only consumers while `ic-blob-storage` is being
qualified. Canic continues to own its existing embedded implementation and wire
contract; this change does not replace it or introduce another service protocol.

## Boundary

Blob storage is for application product data. It is not the canister-snapshot
backup repository, and enabling it does not upload Canic backups. Non-billing
gateway administration also does not imply Cashier authority or monetary
automation.

## Continue From Here

- [Runtime feature selection](../../../crates/canic/README.md#feature-contract)
- [Blob storage integration](../../operations/blob-storage-integration.md)
- [Billing readiness](../../operations/blob-storage-billing-readiness.md)
- [Blob storage inventory contract](../../contracts/BLOB_STORAGE_INVENTORY.md)
- [Cashier inventory contract](../../contracts/BLOB_STORAGE_CASHIER_INVENTORY.md)
- [Choose another feature](../README.md)
- [Browse all documentation](../../README.md)
- [Back to the main README](../../../README.md)
