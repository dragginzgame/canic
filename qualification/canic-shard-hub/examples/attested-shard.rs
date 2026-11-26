//! Observe the routing Root of the installed user shard.
extern crate example_user_directory_shard as _;

#[canic::query]
fn routing_root() -> Option<ic_platform_types::Principal> {
    canic::role::root()
}
