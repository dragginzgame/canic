use super::{Account, Binding, Request, Resource};
use ic_db::Entity;
use ic_platform_types::Principal;

/// Application schema binding for a unique `issuer,subject` index. The bounded
/// lookup admits one row with at most 1 KiB of encoded output.
pub trait Record {
    /// The original entity from the application schema.
    type Row: Entity;

    /// Shared hub/shard protocol.
    type Resource: Resource;

    /// Access this actor's already opened database, synchronously.
    fn with_store<T>(f: impl FnOnce(&mut ic_db::Database) -> T) -> T;

    /// Return the persisted account and ID in the application's Wire representation.
    fn identity(row: &Self::Row) -> (Account, <Self::Resource as Resource>::Id);
}

/// The application's single durable hub route.
pub trait HubRoute: Record {
    /// Scope required to admit new work after suspension.
    const SCOPE: &'static str;
    /// Normalized initial payload as stored by the application.
    type Initial;

    /// Normalize public input before an assignment.
    fn normalize(data: Data<Self>) -> Self::Initial;

    /// Map a new reservation to the existing pending row.
    fn pending(binding: Binding<Id<Self>>, initial: Self::Initial) -> Self::Row;

    /// Original target and payload. Never substitute the retry's payload.
    fn retained(row: &Self::Row) -> (Principal, Data<Self>);

    /// Whether the exact confirmation was committed.
    fn ready(row: &Self::Row) -> bool;

    /// Set the completion flag on this row.
    fn mark_ready(row: &mut Self::Row);
}

/// Application-owned private schema; the adapter handles insert versus replay.
pub trait ShardRecord: Record {
    /// Validate initial data and construct the private row for this account.
    fn create(request: Ticket<Self>) -> Self::Row;
}

/// App-selected Wire ID.
pub type Id<E> = <<E as Record>::Resource as Resource>::Id;
/// App-selected Wire payload.
pub type Data<E> = <<E as Record>::Resource as Resource>::Data;
/// Typed frozen reservation.
pub type Ticket<E> = Request<Id<E>, Data<E>>;
