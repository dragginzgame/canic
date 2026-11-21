//! Compose a receiver lifecycle around the installed topology and retained storage layout.

/// Compose the selected Auth receiver, topology and readiness with the application's database.
/// The parent cell stores the principal from the approved topology envelope.
/// The hub config cell instead retains the routing Root and install context.
#[macro_export]
macro_rules! mount_receiver_lifecycle {
    (config_key = $key:literal, authority = $owner:literal, $(prefix = $prefix:literal,)?
     pages = $pages:literal, auth_read = $scope:literal $(, max_login_secs = $seconds:expr)? $(,)?) => {
        $crate::mount_receiver_lifecycle!(@hooks hub_readiness, $scope,
            open_hub, $owner, $crate::mount_receiver_lifecycle!(@prefix $owner $(, $prefix)?), $key, $pages $(, $seconds)?);
    };
    (parent_storage_key = $key:literal, authority = $owner:literal, $(prefix = $prefix:literal,)?
     $(pages = $pages:literal,)? auth_read = $scope:literal $(, max_login_secs = $seconds:expr)? $(,)?) => {
        $crate::mount_receiver_lifecycle!(@hooks shard_readiness, $scope,
            open_shard, $owner, $crate::mount_receiver_lifecycle!(@prefix $owner $(, $prefix)?), $key, $crate::mount_receiver_lifecycle!(@pages $($pages)?) $(, $seconds)?);
    };
    (@hooks $ready:ident, $scope:literal,
     $open:ident, $owner:literal, $prefix:expr, $key:literal, $pages:expr $(, $seconds:expr)?) => {
        mod receiver_lifecycle {
            use super::database;
            use $crate::role;
            $crate::role::mount_directory!();
            $crate::mount_auth_lifecycle!(storage = open $(, max_login_secs = $seconds)?);
            fn open(initial: Option<&$crate::auth::install::Install>) -> $crate::role::RoleStorage {
                let (host, nonce) = role::$open(initial, $owner, $prefix, &database::keys(), $key, $pages);
                database::attach(&host, nonce.map(|nonce| (nonce, $crate::__private::runtime::system::canister_self())));
                host
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
