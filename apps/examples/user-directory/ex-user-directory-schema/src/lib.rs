//! Shared account schema and the hub-to-shard Wire contract.
pub mod user_hub;
pub mod user_shard;

use canic::sharding::provisioning::Resource;

/// Each signup provisions one private ID, with no additional profile payload.
pub struct Users;

impl Resource for Users {
    type Id = [u8; 16];
    type Data = ();
    const METHOD: &'static str = "provision_user";
    const DOMAIN: &'static [u8] = b"example.user-directory.account.v1";
    const LIMITS: ic_wire::Limits = canic::sharding::LIMITS;
}
