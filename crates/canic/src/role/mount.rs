//! Compose a receiver lifecycle around the installed topology and retained storage layout.

/// Compose volatile Auth, topology and readiness with the application's database.
/// The parent cell stores the principal from the approved topology envelope.
/// The hub config cell instead retains the routing Root and install context.
#[macro_export]
macro_rules! mount_receiver_lifecycle {
    (config_key = $key:literal, authority = $owner:literal, $(prefix = $prefix:literal,)?
     pages = $pages:literal, auth_read = $scope:literal $(,)?) => {
        $crate::mount_receiver_lifecycle!(@hooks HubConfig, decode_hub, hub_readiness, $scope,
            open_hub, $owner, $crate::mount_receiver_lifecycle!(@prefix $owner $(, $prefix)?), $key, $pages);
    };
    (parent_storage_key = $key:literal, authority = $owner:literal, $(prefix = $prefix:literal,)?
     $(pages = $pages:literal,)? auth_read = $scope:literal $(,)?) => {
        $crate::mount_receiver_lifecycle!(@hooks ShardConfig, decode_shard, shard_readiness, $scope,
            open_shard, $owner, $crate::mount_receiver_lifecycle!(@prefix $owner $(, $prefix)?), $key, $crate::mount_receiver_lifecycle!(@pages $($pages)?));
    };
    (@hooks $config:ident, $decode:ident, $ready:ident, $scope:literal,
     $open:ident, $owner:literal, $prefix:expr, $key:literal, $pages:expr) => {
        mod receiver_lifecycle {
            use super::database;
            use $crate::role;
            $crate::role::mount_directory!();
            $crate::mount_auth_lifecycle!(decode = $crate::role::$decode, open = open);
            fn open(initial: Option<role::$config>) {
                let (host, nonce) = role::$open(initial, $owner, $prefix, &database::keys(), $key, $pages);
                database::attach(&host, nonce.map(|nonce| (nonce, $crate::__private::runtime::system::canister_self())));
            }
            $crate::mount_auth_session!(read = $scope);
            $crate::mount_readiness_endpoint!($crate::role::$ready);
        }
    };
    (@prefix $owner:literal) => { concat!($owner, ".") };
    (@prefix $owner:literal, $prefix:literal) => { $prefix };
    (@pages) => { None };
    (@pages $pages:literal) => { Some($pages) };
}
