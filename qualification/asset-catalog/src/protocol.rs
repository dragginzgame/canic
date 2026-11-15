//! Application-owned user and catalog contract; Blob DTOs are used without copies.
use canic::blob::supplier::{
    local::{Error as BlobError, Limits, ObjectView, ReferenceView, Root, UploadRequest},
    references::ReferenceId,
};
use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};

/// Explicit installation configuration; upgrades accept only a method-bound unit.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Installation {
    /// The two users allowed to own assets. Controller status grants no catalog permission.
    pub users: [Principal; 2],
    /// Exact Blob maintenance authority, independently of users and IC controllers.
    pub manager: Principal,
    /// Supplier-owned finite resource limits.
    pub limits: Limits,
}
/// One authorized catalog declaration. Uploader is always the captured caller.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Draft {
    /// Permanent upload and catalog transaction identity.
    pub id: std::num::NonZeroU128,
    /// Permanent application reference identity.
    pub reference: ReferenceId,
    /// Globally unique catalog name, retained after release.
    pub name: String,
    /// Declared content commitment.
    pub root: Root,
    /// Declared content size, not measured physical use.
    pub bytes: u64,
    /// Original exclusive deadline in IC nanoseconds.
    pub expires_at: u64,
    /// Supplier manifest leaves.
    pub chunks: Vec<Root>,
    /// Supplier manifest metadata.
    pub headers: Vec<(String, String)>,
}
/// Application publication decision, independent of Blob registration.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Publication {
    /// Awaiting the owner's decision.
    Draft,
    /// Published by its owner while its reference was live.
    Published,
    /// Removed permanently; historical replay cannot publish it again.
    Released,
}
/// Durable application binding; not a second Blob accounting ledger.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Entry {
    /// Application name.
    pub name: String,
    /// Original authorization, including the actual uploader captured at admission.
    pub request: UploadRequest,
    /// Application-owned publication decision.
    pub publication: Publication,
}
/// Authorized inspection of this user's application use.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Asset {
    /// Application decision.
    pub entry: Entry,
    /// Current Blob reference, inspected locally after the user check.
    pub reference: ReferenceView,
    /// Shared object's current retention, not a provider availability proof.
    pub object: ObjectView,
}
/// Ordinary refusals precede mutation. Later persistence/encoding failures trap.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Error {
    /// Not an admitted user or not this asset's owner.
    Denied,
    /// Invalid name or inconsistent catalog declaration.
    Invalid,
    /// Another immutable catalog entry already owns this name/identity.
    Conflict,
    /// Finite lifetime catalog capacity exhausted.
    Capacity,
    /// Unknown application reference.
    Missing,
    /// Released assets cannot be republished.
    Released,
    /// Publication requires a currently live reference.
    NotLive,
    /// Supplier-owned refusal.
    Blob(BlobError),
}
/// Lifetime catalog slots; released entries remain occupied.
pub const CAPACITY: usize = 4;
/// Explicit installation frame.
pub const INIT: u64 = ic_wire::native::method_id("catalog_init");
/// Unit-only upgrade frame.
pub const UPGRADE: u64 = ic_wire::native::method_id("catalog_upgrade");
/// Authorize a declaration and retain its catalog binding atomically.
pub const AUTHORIZE: u64 = ic_wire::native::method_id("catalog_authorize");
/// Publish an owner's live use.
pub const PUBLISH: u64 = ic_wire::native::method_id("catalog_publish");
/// Release the user's use and remove publication atomically.
pub const REMOVE: u64 = ic_wire::native::method_id("catalog_remove");
/// Inspect only the requesting user's use.
pub const INSPECT: u64 = ic_wire::native::method_id("catalog_inspect");
/// Qualification-only failure selector, available only in this test application.
pub const FAULT: u64 = ic_wire::native::method_id("catalog_fault");
