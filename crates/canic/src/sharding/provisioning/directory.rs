use super::{Account, Binding, Request, Resource};
use ic_db::{Database, Entity};
use ic_platform_types::Principal;

/// Application schema binding for a unique `issuer,subject` index. The bounded
/// lookup admits one row within a 1 KiB read budget.
pub trait Record: Entity {
    /// Shared hub/shard protocol.
    type Resource: Resource;

    /// Access this actor's already opened database, synchronously.
    fn with_store<T>(f: impl FnOnce(&mut Database) -> T) -> T;

    /// Return the persisted account and ID in the application's Wire representation.
    fn identity(&self) -> (Account, <Self::Resource as Resource>::Id);
}

/// The application's single durable hub route.
pub trait HubRoute: Record {
    /// Scope required to admit new work after suspension.
    const SCOPE: &'static str;
    /// Normalized initial payload as stored by the application.
    type Initial;

    /// Normalize public input before an assignment.
    fn normalize(data: Data<Self>) -> Self::Initial;

    /// Generate a candidate ID using the application's policy.
    fn new_id() -> impl Future<Output = Id<Self>>;

    /// Map a new reservation to the existing pending row.
    fn pending(binding: Binding<Id<Self>>, initial: Self::Initial) -> Self;

    /// Original target and payload. Never substitute the retry's payload.
    fn retained(&self) -> (Principal, Data<Self>);

    /// Whether the exact confirmation was committed.
    fn ready(&self) -> bool;

    /// Set the completion flag on this row.
    fn mark_ready(&mut self);
}

/// Application-owned private schema; the adapter handles insert versus replay.
pub trait ShardRecord: Record {
    /// Validate initial data and construct the private row for this account.
    fn create(request: Ticket<Self>) -> Self;
}

/// App-selected Wire ID.
pub type Id<E> = <<E as Record>::Resource as Resource>::Id;
/// App-selected Wire payload.
pub type Data<E> = <<E as Record>::Resource as Resource>::Data;
/// Typed frozen reservation.
pub type Ticket<E> = Request<Id<E>, Data<E>>;

/// Bind a conventional IC DB account row to the installed hub or shard.
/// The table declares a unique `issuer,subject` index and the app supplies the
/// ID codec and validated payload. The hub status and payload remain in its
/// original row. This declaration mounts no endpoint or lifecycle hook.
#[macro_export]
macro_rules! declare_provisioning_row {
    (@record $row:ident, $resource:ty, $store:path, $encode:path) => {
        impl $crate::sharding::provisioning::Record for $row {
            type Resource = $resource;

            fn with_store<T>(f: impl FnOnce(&mut ::ic_db::Database) -> T) -> T { $store(f) }

            fn identity(&self) -> ($crate::sharding::provisioning::Account, $crate::sharding::provisioning::Id<Self>) {
                ($crate::sharding::provisioning::Account {issuer: self.issuer, subject: self.subject}, $encode(self.id))
            }
        }
    };
    (hub $row:ident, resource = $resource:ty, store = $store:path,
     id = ($encode:path, $decode:path, $generate:path), scope = $scope:literal,
     payload = $field:ident: $initial:ty, validate = $validate:path, export = $export:expr,
     status = ($pending:path, $ready:path) $(,)?) => {
        $crate::declare_provisioning_row!(@record $row, $resource, $store, $encode);
        impl $crate::sharding::provisioning::HubRoute for $row {
            const SCOPE: &'static str = $scope;
            type Initial = $initial;

            fn normalize(data: $crate::sharding::provisioning::Data<Self>) -> Self::Initial { $validate(data) }

            async fn new_id() -> $crate::sharding::provisioning::Id<Self> { $encode($generate().await) }

            fn pending(binding: $crate::sharding::provisioning::Binding<$crate::sharding::provisioning::Id<Self>>, initial: Self::Initial) -> Self {
                Self { id: $decode(binding.id), issuer: binding.account.issuer, subject: binding.account.subject,
                       shard: binding.shard, $field: initial, status: $pending }
            }

            fn retained(&self) -> (::ic_db::Principal, $crate::sharding::provisioning::Data<Self>) {
                (self.shard, ($export)(&self.$field))
            }

            fn ready(&self) -> bool { self.status == $ready }

            fn mark_ready(&mut self) { self.status = $ready; }
        }
    };
    (shard $row:ident, resource = $resource:ty, store = $store:path,
     id = ($encode:path, $decode:path), payload = $field:ident, validate = $validate:path $(,)?) => {
        $crate::declare_provisioning_row!(@record $row, $resource, $store, $encode);
        impl $crate::sharding::provisioning::ShardRecord for $row {
            fn create(request: $crate::sharding::provisioning::Ticket<Self>) -> Self {
                Self { id: $decode(request.binding.id), issuer: request.binding.account.issuer,
                       subject: request.binding.account.subject, $field: $validate(request.data) }
            }
        }
    };
}
