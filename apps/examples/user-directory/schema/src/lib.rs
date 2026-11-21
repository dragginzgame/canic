//! Shared account schema and the hub-to-shard Wire contract.
pub mod model;

ic_db::include_schema!();

use canic::sharding::provisioning::Resource;

///
/// Users
/// Each signup provisions one private ID, with no additional profile payload.
///

pub struct Users;

impl Resource for Users {
    type Id = [u8; 16];
    type Data = ();
    type Refusal = ProvisionRefusal;
    const METHOD: &'static str = "provision_user";
    const DOMAIN: &'static [u8] = b"example.user-directory.account.v1";
}

///
/// ProvisionRefusal
/// Reasons the directory shard declined a registration.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
pub enum ProvisionRefusal {
    /// The account, ID or destination differs from the requested binding.
    BindingConflict,
    /// The account row could not be inserted.
    Database,
}

/// Retry-safe reservation of an account's project identifier at the named service.
#[cfg(feature = "role-service")]
pub struct ReserveProject;

#[cfg(feature = "role-service")]
impl canic::method::Contract for ReserveProject {
    const NAME: &'static str = "reserve_project";
    const ID: u64 = canic::method::wire::method_id(Self::NAME);
    type Input = canic::sharding::provisioning::AuthSubject;
    type Output = [u8; 16];
}
#[cfg(feature = "role-service")]
impl canic::topology::attestation::RoleMethod for ReserveProject {
    const ROLES: &'static [&'static str] = &["user_shard"];
}
