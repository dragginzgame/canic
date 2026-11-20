//! Compose the declared database, Auth receiver, topology and readiness lifecycle.

/// Select one store from the schema owner and mount an Auth-enrolled hub or shard.
/// Pass the store's crate-qualified type path in the role setup module.
/// Select `stateful-role` for the mounted actor on both targets; plain stores use `db-store`.
/// A shard retains its approved parent in `<prefix>hub.v1`. For a predecessor
/// using a different allocation, specify its exact `parent_storage_key`.
#[macro_export]
macro_rules! mount_db_role {
    (hub, database = $store:path, config = $config:literal, authority = $owner:literal,
     $(prefix = $prefix:literal,)? pages = $pages:literal, auth_read = $scope:literal $(,)?) => {
        $crate::declare_db_store!(@typed pub mod database, $store, {
            /// Stable hub configuration allocation.
            pub const CONFIG_KEY: &str = $config;
            /// Storage allocation owner.
            pub const AUTHORITY: &str = $owner;
            /// Namespace admitted by the Storage owner.
            pub const PREFIX: &str = $crate::mount_db_role!(@prefix $owner $(, $prefix)?);
            /// Maximum pages in the Storage directory.
            pub const PAGES: u64 = $pages;
            /// Scope required to read an Auth session.
            pub const AUTH_READ: &str = $scope;

            $crate::__canic_mount_db_actor!(HubConfig, decode_hub, hub_readiness, initial, {
                role::open_hub(initial, AUTHORITY, PREFIX, DATABASE_KEY, CONFIG_KEY, PAGES, store::attach);
            });
        });
    };
    (shard, database = $store:path, authority = $owner:literal,
     prefix = $prefix:literal, $(pages = $pages:literal,)? auth_read = $scope:literal $(,)?) => {
        $crate::mount_db_role!(
            shard,
            database = $store,
            parent_storage_key = concat!($prefix, "hub.v1"),
            authority = $owner,
            prefix = $prefix,
            $(pages = $pages,)?
            auth_read = $scope
        );
    };
    (shard, database = $store:path, parent_storage_key = $parent:expr, authority = $owner:literal,
     prefix = $prefix:literal, $(pages = $pages:literal,)? auth_read = $scope:literal $(,)?) => {
        $crate::declare_db_store!(@typed pub mod database, $store, {
            /// Stable allocation for the installed parent.
            pub const PARENT_STORAGE_KEY: &str = $parent;
            /// Storage allocation owner.
            pub const AUTHORITY: &str = $owner;
            /// Namespace admitted by the Storage owner.
            pub const PREFIX: &str = $prefix;
            /// Optional override of the Storage directory's page budget.
            pub const PAGES: Option<u64> = $crate::mount_db_role!(@pages $($pages)?);
            /// Scope required to read an Auth session.
            pub const AUTH_READ: &str = $scope;

            $crate::__canic_mount_db_actor!(ShardConfig, decode_shard, shard_readiness, initial, {
                role::open_shard(initial, AUTHORITY, PREFIX, DATABASE_KEY, PARENT_STORAGE_KEY, PAGES, store::attach);
            });
        });
    };
    (@prefix $owner:literal) => { concat!($owner, ".") };
    (@prefix $owner:literal, $prefix:literal) => { $prefix };
    (@pages) => { None };
    (@pages $pages:literal) => { Some($pages) };
}

// Older manifests select only db-store on native and stateful-role on Wasm.
// Select the implementation in Canic, where that feature is declared.
#[cfg(feature = "stateful-role")]
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_mount_db_actor {
    ($config:ident, $decode:ident, $ready:ident, $initial:ident, $open:block) => {
        use $crate::role;
        $crate::role::mount_directory!();

        $crate::mount_auth_lifecycle!(
            decode_context = $crate::role::$decode,
            open = super::open,
        );

        fn open($initial: Option<role::$config>) $open

        $crate::mount_auth_session!(read = super::AUTH_READ);
        $crate::mount_readiness_endpoint!($crate::role::$ready);
    };
}

#[cfg(not(feature = "stateful-role"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_mount_db_actor {
    ($($role:tt)*) => {
        #[cfg(target_arch = "wasm32")]
        compile_error!("mount_db_role! requires canic/stateful-role for a deployed actor");
    };
}
