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
    const METHOD: &'static str = "provision_user";
    const DOMAIN: &'static [u8] = b"example.user-directory.account.v1";
}
