# Blob Storage Source Handoff

This runbook is for the 0.69 blob-storage inventory-unlock work.

The 0.69 implementation remains blocked until the gateway inventory is
completed from source-backed protocol evidence. Toko files are useful
consumer/wrapper evidence, but they are not automatically authoritative
gateway or Cashier protocol source.

## Current Evidence

GitHub code search found candidate Toko files at commit, and the local Toko
repository now has that commit available on `origin/development`:

```text
3ef01afc5f5eeefdb9471f3e010b6562d758c111
```

The candidate commit was inspected locally with `git show` / `git grep`
without checking it out.

The local Toko checkout `HEAD` observed after the maintainer pull is:

```text
/home/adam/projects/toko
branch: boss
commit: 97aafee9eeb73ae0517f9788df688bb96ae0a9ff
state: user-managed; later status showed unmerged pull/conflict entries
```

The checked-out `boss` `HEAD` does not contain the blob-storage method
literals. The working tree may contain conflict contents from user activity, so
use `git show` against the candidate commit, or a separate clean checkout, when
inspecting the development blob-storage source.

## Safe Inspection Path

Use a clean checkout, authenticated source path, or read-only `git show` from
the local Toko repository.

Example with a temporary clone:

```bash
git clone https://github.com/dragginzgame/toko /tmp/toko-blob-source
git -C /tmp/toko-blob-source checkout 3ef01afc5f5eeefdb9471f3e010b6562d758c111
```

If the repository is private, first fix authentication with the maintainer's
normal GitHub flow. Do not commit credentials or generated private artifacts
into Canic.

Example from the local Toko repository without changing its worktree:

```bash
git -C /home/adam/projects/toko show 3ef01afc5f5eeefdb9471f3e010b6562d758c111:fleets/toko/project/instance/project_instance.did
```

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
  compatibility evidence.

The Toko project-instance canister endpoint source is now recorded in
`docs/contracts/BLOB_STORAGE_INVENTORY.md` as source-identified Toko evidence.
Cashier remains consumer/wrapper evidence only until actual Cashier source or
generated/deployed Cashier Candid is found.

## Required Updates

When source is inspected, update:

- `docs/contracts/BLOB_STORAGE_INVENTORY.md`
- `docs/contracts/BLOB_STORAGE_CASHIER_INVENTORY.md`
- `docs/design/0.69-blob-storage/0.69-design.md` only if source contradicts
  the current implementation plan

Do not add `blob-storage` feature metadata, DTOs, Candid snapshots, stable
records, endpoint literals, macros, or behavior tests until the relevant
inventory is complete and the gate accepts it.

## Validation

After updating evidence, run:

```bash
bash scripts/ci/check-blob-storage-inventory-gate.sh
bash scripts/ci/check-blob-storage-cashier-inventory-gate.sh
cargo test --locked -p canic --test protocol_inventory_gate -- --nocapture
```
