# Blob Storage

Canic exposes optional runtime and operator integrations for product blob
storage. The base feature covers local gateway state and administration; the
billing feature adds Cashier-backed status, funding, and readiness flows.

## What It Provides

- opt-in `blob-storage` runtime APIs and endpoint macros
- controller-guarded gateway administration
- stable local counters and root-hash state
- opt-in `blob-storage-billing` Cashier integration
- operator status, gateway synchronization, funding, and medic checks
- separate Cargo features so ordinary canisters carry none of this surface

Downstream canisters select the feature explicitly and choose the endpoint
guard appropriate to their application authority.

## Boundary

Blob storage is for application product data. It is not the canister-snapshot
backup repository, and enabling it does not upload Canic backups. Non-billing
gateway administration also does not imply Cashier authority or monetary
automation.

## Start Here

- [Runtime feature selection](../../../crates/canic/README.md#feature-contract)
- [Blob storage integration](../../operations/blob-storage-integration.md)
- [Billing readiness](../../operations/blob-storage-billing-readiness.md)
- [Blob storage inventory contract](../../contracts/BLOB_STORAGE_INVENTORY.md)
- [Cashier inventory contract](../../contracts/BLOB_STORAGE_CASHIER_INVENTORY.md)
