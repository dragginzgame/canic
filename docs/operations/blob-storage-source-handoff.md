# Blob Storage Source Handoff

This runbook records the 0.69 blob-storage inventory-unlock evidence.

The 0.69 gateway inventory is complete. Current Toko `boss` is accepted as the
project-side protocol source for 0.69 implementation work. Cashier remains
consumer/wrapper evidence only until actual Cashier source or generated/deployed
Cashier Candid is found for 0.70 billing work.

## Current Evidence

The local Toko repository now has the blob-storage project-instance source on
the checked-out `boss` branch:

```text
/home/adam/projects/toko
branch: boss
commit: 9ca150b396a2bde42f2b8977a04a7ca2c6172b56
state: clean at 2026-06-20 inspection
```

This supersedes the earlier 2026-06-19 local-`boss` finding, where the
checked-out branch lacked the blob-storage method literals.

Historical GitHub/development evidence remains useful for audit continuity:

```text
3ef01afc5f5eeefdb9471f3e010b6562d758c111
```

That commit was previously inspected locally with `git show` / `git grep`
without checking it out. The current `boss` commit is the source identifier for
0.69 implementation.

## Safe Inspection Path

Use the clean local Toko checkout first. For source snippets, prefer read-only
commands against the current committed `HEAD`:

```bash
git -C /home/adam/projects/toko grep -n "_immutableObjectStorage" HEAD -- .
git -C /home/adam/projects/toko show HEAD:fleets/toko/project/instance/project_instance.did
```

If the local checkout becomes dirty or unavailable, use a separate clean clone
or a read-only `git show` against commit
`9ca150b396a2bde42f2b8977a04a7ca2c6172b56`. If the repository is private,
first fix authentication with the maintainer's normal GitHub flow. Do not
commit credentials or generated private artifacts into Canic.

## Inspected Toko Files

Gateway-facing project-instance candidates:

- `fleets/toko/project/instance/src/lib.rs`
- `fleets/toko/project/instance/project_instance.did`
- `frontend/src/generated/declarations/project_instance/project_instance.did.js`
- `frontend/src/generated/declarations/project_instance/project_instance.did.d.ts`
- `frontend/src/lib/storage/storage-client.ts`

Project-hub status/billing candidates:

- `fleets/toko/project/hub/src/ops/blob_storage.rs`
- `backend/src/canisters/project/hub/src/ops/blob_storage.rs`
- `fleets/toko/project/hub/project_hub.did`
- `frontend/src/generated/declarations/project_hub/project_hub.did.js`
- `frontend/src/generated/declarations/project_hub/project_hub.did.d.ts`
- `fleets/toko/project/instance/src/ops/immutable_storage.rs`

## Evidence Classification

Record each finding under the correct evidence class:

- Gateway protocol source: actual implementation or generated/deployed `.did`
  for `_immutableObjectStorage*` methods.
- Cashier protocol source: actual implementation or generated/deployed `.did`
  for `account_balance_get_v1`, `account_top_up_v1`, and
  `storage_gateway_principal_list_v1`.
- Toko consumer evidence: downstream caller, wrapper, generated declaration, or
  interoperability evidence.

The Toko project-instance canister endpoint source is now recorded in
`docs/contracts/BLOB_STORAGE_INVENTORY.md` as complete 0.69 protocol evidence.
Cashier remains consumer/wrapper evidence only until actual Cashier source or
generated/deployed Cashier Candid is found.

## Required Updates

When source evidence changes, update:

- `docs/contracts/BLOB_STORAGE_INVENTORY.md`
- `docs/contracts/BLOB_STORAGE_CASHIER_INVENTORY.md`
- `docs/design/0.69-blob-storage/0.69-design.md` only if source contradicts
  the current implementation plan

The 0.69 gateway inventory is complete, so non-billing `blob-storage`
implementation work may proceed. Do not add `blob-storage-billing`, Cashier
wrappers, gateway-principal sync from Cashier, funding, or status surfaces
until the Cashier inventory is complete and the separate Cashier gate accepts
it.

## Validation

After updating evidence, run:

```bash
bash scripts/ci/check-blob-storage-inventory-gate.sh
bash scripts/ci/check-blob-storage-cashier-inventory-gate.sh
cargo test --locked -p canic --test protocol_inventory_gate -- --nocapture
```
