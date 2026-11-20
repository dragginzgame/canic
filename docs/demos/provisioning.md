# Migrating Toko signup to ordinary application rows

The row-based provisioning API first shipped in Canic **0.237.0**. The
scoped handler signatures below require the unreleased handler-local Auth change.
The original provisioning implementation was tested at `9af0094d6e6950e61e4ce245b9d912b8e75e33cb`.
Version 0.236.0 contains the previous API.

The migration targets Toko source `3ea25ea2d48d9c47356c99a8c585a7120e9506e3`.
Its hub owns the ID, initial name and pending/ready transition. Its shard owns
the current name and account record. Canic owns authenticated placement, the
parent and destination checks, transport and verification of the acknowledgement.

## Replace the two handlers

Previously the hub and shard delegated their entire bodies to these calls.

```rust
// Hub, under #[canic::update(scope = Hub::SCOPE)]
provisioning::provision::<Hub>(&session, name, provisioning::new_ulid).await

// Shard, under the Provision<Users> contract and require_parent guard
provisioning::receive::<Shard>(request)
```

The **complete replacement Rust handlers** below use Toko's real entities and
display-name policy. Merge their imports with the existing module imports and
keep the other endpoints in each module.

| In toko-new | Edit |
| --- | --- |
| `apps/toko/user_hub/src/endpoints/outbound.rs` | Replace `register` with [the after implementation, scope and binding helper](../../qualification/fixtures/provisioning-roles/register.rs). Keep the two project/authority endpoints and their imports. |
| `apps/toko/user_shard/src/endpoints/mod.rs` | Replace `provision_user` with [the after implementation](../../qualification/fixtures/provisioning-roles/provision_user.rs). Keep the contract attribute and its pre-decode parent guard. |
| Both roles' `src/ops/provisioning.rs` | Delete both files. This removes `Hub`, `Shard`, both `Record` implementations, `HubRoute` and `ShardRecord`. Remove `mod provisioning;` from both `src/ops/mod.rs` files. |
| Both roles' `src/lib.rs` | Remove the `Hub` and `Shard` re-exports. Keep the existing role mount, database module, row types and remaining exports. |
| `crates/toko/src/user/provisioning.rs` | Keep `Users: Resource`. It describes the existing Wire call and placement-key domain. It contains no persistence hooks. |
| Toko schema and user operations | Keep the existing entities, indexes, validation, profile reads/edits and private authorization. Keep the scoped read and write endpoints. |

## Public API

All names below live in `canic::sharding::provisioning`. Enable the existing
`provisioning` feature on the actor graph. `Request`, `Binding`, `Resource`,
`Provision<R>` and `Progress` retain their Wire definitions.

```rust
pub async fn place<'a, R: Resource>(
    auth: &'a Authorization,
    retained: Option<Principal>,
) -> Result<Assignment<'a, R>, Progress>;

impl<R: Resource> Assignment<'_, R> {
    pub fn shard(&self) -> Principal;
    pub async fn send(self, request: &Request<R::Id, R::Data>)
        -> Result<Binding<R::Id>, Progress>;
}

impl<I> Binding<I> {
    pub fn require_target(&self);
}
impl<I: Eq> Binding<I> {
    pub fn require_same(&self, retained: &Self);
}

pub fn account_row<E: Entity>(database: &DatabaseSession, account: Account) -> Option<E>;
pub fn session_row<E: Entity>(database: &DatabaseSession, auth: &Authorization) -> Option<E>;
```

1. Validate the name synchronously, including on retries. An already-ready row
   can return locally under the endpoint's write scope. Pass `&auth` to
   `place`; it retains the scope declared on the endpoint. Supply the original shard for a pending row. That selects lookup
   only. On a miss, Root's assign first looks for an existing claim for that key.
2. After placement, reuse the original row or reload the winner before inserting.
   Generate `Ulid::generate_prand()` only in the missing-row branch. The unique
   account lookup, insert and construction of the request execute without an
   await. Send the stored ID, shard and initial name, even when the retry has
   supplied a different name.
3. `send` verifies the account and destination, checks Root's original slot,
   generation and principal before dispatch and after the receipt, and rechecks
   the session after suspension. Its successful result has already matched the
   original request. Reload the local reservation, call `receipt.require_same`
   and update that row synchronously. Keep the interval between verification
   and the database write free of awaits.
4. At the shard, call `binding.require_target()` before touching the database.
   Validate the initial name even on replay. Match the immutable ID for the
   bound account. Insert only when the unique lookup misses. A matching retry
   acknowledges the original binding without replacing the current profile.

The two optional IC DB readers use Toko's existing unique `issuer,subject`
index and retain its one-row, 1 KiB encoded-result check. Applications with
different persistence or indexes can perform their own lookup. The placement
and transport APIs take no schema, database handle, callback or field names.

| Outcome | What the handler can establish |
| --- | --- |
| `PlacementPending` | Root reports an in-progress claim or busy creation. |
| `ProvisionPending` | A reservation exists and the shard call produced no usable acknowledgement. Retry its original binding and payload. |
| `PoolFull` | The approved pool's capacity is exhausted. |
| `PlacementUnavailable` | Root or the original target/allocation could not be verified. Preserve any pending reservation. |
| `Ready` | An existing ready row was read, or the matching reservation was confirmed. |

A conflicting decoded acknowledgement or lost write authority traps. The pending
reservation survives. Controller membership does not authorize signup.

## Compatibility and qualification

The change is a Rust API hard cut. The published method names/IDs, directional
Wire bytes and budgets, placement key (domain plus length-prefixed issuer and
subject), HRW selector, install envelopes, fleet configuration, database leases,
schemas and stored row bytes are unchanged. Toko continues to use a native ULID
in its database and `[u8; 16]` on the existing Wire boundary.

From Canic with a prepared local simulator:

```sh
cargo test --locked -p canic --features provisioning sharding::
bash scripts/dev/test-store.sh user-directory
bash scripts/dev/test-store.sh sharding
```

The installed User Directory test runs a real Root, hub and two managed shards.
It checks overlapping registrations, bounded capacity, independent Auth, a
stopped shard, an unavailable Root and replay with a pending hub and a populated
shard. The replay leaves the shard's stable memory byte-for-byte unchanged.
A controlled responder at the disposable shard then returns a wrong ID and,
separately, the correct ID after logout. Neither response changes the pending
hub. The native IC DB test also reopens a populated record after editing its
name and checks a stale local confirmation against the winning reservation.

### Disposable Toko rehearsal

In a separate copy of the Toko source identified above, both role libraries
compiled natively and as IC Wasm. The installed `signup` and `user_shard`
cases passed on fresh installations and using these copied predecessor Wasms:

| Role | Original SHA-256 |
| --- | --- |
| Hub | `4cda0d86c2dac8399f209759c437c8b870a32c479b879942994150da384fe988` |
| Shard | `c9bf11a2f6168eee07881c004f0779b32c933e6520676593a4cd01b4fc8c464b` |

The rehearsal extended signup with the direct parent retry after rename
described below. The private profile still contained “Alice B” and the shard
stable bytes were identical. It used Toko's prepared Root, Store and fixture
issuer Wasms on PocketIC. Real Internet Identity login and production rollout
remain separate qualification paths.

### Run in Toko after migration

Select `canic = "=0.237.0"` in both workspace dependency tables and update both lockfiles. For a
local sibling trial, add this override at the workspace roots (paths relative
to the manifest), then reconcile both lockfiles with Cargo and verify the
resolved `canic` source with `cargo tree -i canic` in each workspace.

```toml
# toko-new/Cargo.toml
[patch.crates-io]
canic = { path = "../canic/crates/canic" }

# toko-new/qualification/Cargo.toml (separate workspace)
[patch.crates-io]
canic = { path = "../../canic/crates/canic" }
```

From Toko's root with the prepared simulator, first copy the two original Wasms
to an external predecessor directory before rebuilding. Run with the new actors,
then repeat against the populated predecessor path.

```sh
export IC_TESTKIT_SERVER_DIRECTORY="$(cd ../canic && pwd)/target/qualification/server/prepared"
cargo test --locked -p user_hub -p user_shard
make test-signup test-user-shard
TOKO_PREDECESSOR_FLEET_DIR=/absolute/path/to/predecessor make test-signup test-user-shard
```

Keep the rename/replay regression in Toko's installed signup suite. After Alice
renames her profile, send the original `Provision<Users>` request from the hub
to her shard, assert the returned binding and compare both her current name and
the shard's stable bytes. A repeat call to a ready hub can return locally and
does not exercise this shard branch. Toko owns the final adoption run with its
selected lockfiles, actual predecessor Wasms and private account assertions.
