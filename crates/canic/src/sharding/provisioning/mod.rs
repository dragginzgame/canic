//! Bounded account provisioning through the installed hub and its managed shards.

#[cfg(all(test, feature = "provisioning", not(target_arch = "wasm32")))]
mod tests;

use crate::method::Contract;
use ic_platform_types::Principal;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// Durable account selector derived from the authenticated issuer and subject.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Account {
    /// Admitted issuer.
    pub issuer: Principal,
    /// Stable subject within this issuer.
    pub subject: Principal,
}

#[cfg(all(feature = "provisioning", target_arch = "wasm32"))]
impl Account {
    fn valid(self) -> bool {
        self.issuer != Principal::anonymous() && self.subject != Principal::anonymous()
    }
}

#[cfg(all(feature = "provisioning", any(test, target_arch = "wasm32")))]
fn matches_target(
    observation: &super::Placement,
    allocation: &super::Allocation,
    shard: Principal,
) -> bool {
    observation.assigned_target().ok().flatten() == Some((allocation, shard))
}

#[cfg(all(feature = "provisioning", any(test, target_arch = "wasm32")))]
fn placement_key<R: Resource>(account: Account) -> super::Key {
    use sha2::{Digest, Sha256};
    let mut digest = Sha256::new().chain_update(R::DOMAIN);
    for principal in [account.issuer, account.subject] {
        let bytes = principal.as_slice();
        digest.update([u8::try_from(bytes.len()).expect("principal length")]);
        digest.update(bytes);
    }
    digest.finalize().into()
}

/// Behavior selected by an application mount, independently of its role name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProvisioningRole {
    /// Reserve, route and confirm accounts.
    Hub,
    /// Validate and store an account for the configured parent.
    Shard,
}

/// Conventional ULID resource with a validated application-owned payload.
pub trait ProvisioningPayload: Resource<Id = [u8; 16]> {
    /// Value stored in the existing application schema.
    type Stored;
    /// Normalize the incoming payload or reject it before mutation.
    fn normalize(input: Self::Data) -> Self::Stored;
    /// Reconstruct the Wire payload from the retained hub reservation.
    fn export(value: &Self::Stored) -> Self::Data;
}

/// The app selects its wire ID, initial payload, method name and key namespace.
pub trait Resource {
    /// App-owned stable ID in its Wire representation.
    type Id: Serialize + DeserializeOwned + Clone + Eq;
    /// Initial data retained by the hub until the original operation is confirmed.
    type Data: Serialize + DeserializeOwned;
    /// Published parent-only update export.
    const METHOD: &'static str;
    /// Domain separator for the permanent placement key.
    const DOMAIN: &'static [u8];
    /// Directional frame budget for this small request and acknowledgement.
    const LIMITS: ic_wire::Limits;
}

/// Exact identity acknowledged by the originally called shard.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Binding<I> {
    /// Application-supplied ID.
    pub id: I,
    /// Account owning this ID.
    pub account: Account,
    /// Admitted target principal.
    pub shard: Principal,
}

/// Exact provisioning request. Retries use the retained payload.
#[derive(Serialize, Deserialize)]
pub struct Request<I, D> {
    /// Immutable assignment.
    pub binding: Binding<I>,
    /// Application-specific initial data.
    pub data: D,
}

/// Typed parent-to-shard protocol.
pub struct Provision<R>(std::marker::PhantomData<R>);
impl<R: Resource> Contract for Provision<R> {
    const NAME: &'static str = R::METHOD;
    const ID: u64 = ic_wire::method_id(R::METHOD);
    const REQUEST: ic_wire::Limits = R::LIMITS;
    const REPLY: ic_wire::Limits = R::LIMITS;
    type Input = Request<R::Id, R::Data>;
    type Output = Binding<R::Id>;
}

/// Result of one bounded signup attempt, in the existing Toko Wire order.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Progress {
    /// The hub holds an exact confirmation.
    Ready,
    /// An earlier creation or capacity claim is pending.
    PlacementPending,
    /// The retained request can be reconciled on retry.
    ProvisionPending,
    /// The approved pool cannot accept another key.
    PoolFull,
    /// The pool cannot verify the original target.
    PlacementUnavailable,
}

#[cfg(feature = "db-store")]
mod directory;

#[cfg(all(feature = "provisioning", any(test, target_arch = "wasm32")))]
mod rows;
#[cfg(feature = "db-store")]
pub use directory::{Data, HubRoute, Id, Record, ShardRecord, Ticket};
#[cfg(all(feature = "provisioning", target_arch = "wasm32"))]
mod actor;
#[cfg(all(feature = "provisioning", target_arch = "wasm32"))]
pub use actor::{provision, receive};
