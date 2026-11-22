//! Compose a receiver lifecycle around the installed topology and retained storage layout.

/// Compose the selected Auth receiver, topology and readiness.
/// Select `hub` for a service without a database. Its optional `init(&Install)` and
/// `post_upgrade()` hooks run after Canic initializes or restores the receiver.
/// `pre_upgrade()` runs after the receiver seals its checkpoint. Reinstall runs init.
/// Omit `auth_read` to omit the portable session endpoints. Reserved keys preserve
/// an existing storage directory when an application stops using an allocation.
/// Every hub opens Canic’s durable shard assignments in its Storage bootstrap.
/// The parent cell stores the principal from the approved topology envelope.
/// The hub config cell instead retains the routing Root and install context.
#[macro_export]
macro_rules! mount_receiver_lifecycle {
    (hub, config_key = $key:literal, authority = $owner:literal,
     $(reserved_keys = [$($reserved:literal),* $(,)?],)?
     $(auth_read = $scope:expr,)? $(max_login_secs = $seconds:expr,)?
     $(init = $init:path,)? $(pre_upgrade = $pre:path,)? $(post_upgrade = $post:path,)? ) => {
        $crate::mount_hub_routes!();
        $crate::role::mount_directory!();
        const _: () = {
            fn hub_storage(install: Option<&$crate::auth::install::Install>) -> $crate::role::RoleStorage {
                let (storage, _) = $crate::role::open_hub(
                    install, $owner, concat!($owner, "."), &[$($($reserved),*)?], $key, 65_536,
                );
                storage
            }
            $crate::mount_auth_lifecycle!(
                storage = hub_storage
                $(, max_login_secs = $seconds)?
                $(, init = $init)?
                $(, pre_upgrade = $pre)?
                $(, post_upgrade = $post)?
            );
        };
        $( $crate::mount_auth_session!(read = $scope); )?
        $crate::mount_readiness_endpoint!($crate::role::hub_readiness);
    };
    (config_key = $key:literal, authority = $owner:literal, $(prefix = $prefix:literal,)?
     pages = $pages:literal, auth_read = $scope:literal $(, max_login_secs = $seconds:expr)? $(,)?) => {
        $crate::mount_hub_routes!();
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
