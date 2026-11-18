//! Install or reopen one typed database after the actor admits its complete Storage directory.
use ic_db::Schema;
use ic_storage::{Memory, StableKey};
use std::cell::RefCell;

/// Declare in actor-local state. The key and schema must match the retained database.
pub struct Store<M: Memory = DefaultMemoryImpl> {
    key: &'static str,
    schema: fn() -> Schema,
    database: RefCell<Option<Database<M>>>,
}

impl<M: Memory> Store<M> {
    /// Declare the store without allocating memory, opening a database or exporting methods.
    pub const fn new(key: &'static str, schema: fn() -> Schema) -> Self {
        Self {
            key,
            schema,
            database: RefCell::new(None),
        }
    }

    /// Attach the database to this heap. Fresh installation supplies its incarnation.
    /// Upgrade passes `None` and strictly opens the existing bytes.
    ///
    /// # Panics
    /// Traps on duplicate initialization, missing allocation, or any schema or DB refusal.
    /// Call from the lifecycle message so its failure rolls back adjacent state changes.
    pub fn attach(&self, host: &MemoryRuntime<M>, incarnation: Option<[u8; 16]>) {
        let mut slot = self.database.borrow_mut();
        assert!(slot.is_none(), "database already attached: {}", self.key);
        let key = StableKey::parse(self.key).expect("database allocation key");
        let memory = host.open_memory(&key).expect("database allocation");
        let schema = (self.schema)();
        *slot = Some(match incarnation {
            Some(id) => Database::create(memory, schema, id).expect("create declared database"),
            None => Database::open(memory, schema).expect("reopen declared database"),
        });
    }

    /// Borrow the attached database for one synchronous application operation.
    ///
    /// # Panics
    /// Traps before attachment or on a nested borrow of the same store.
    pub fn with<R>(&self, operation: impl FnOnce(&mut Database<M>) -> R) -> R {
        operation(
            self.database
                .borrow_mut()
                .as_mut()
                .expect("database lifecycle has not run"),
        )
    }
}

#[doc(hidden)]
pub use ic_db::Database;
#[doc(hidden)]
pub use ic_storage::{DefaultMemoryImpl, MemoryRuntime};

/// Declare an actor-local database module with `attach` and synchronous `with`.
/// Supply only the allocation key to bind IC DB's generated `OUT_DIR/schema.rs`
/// from the consuming package. Native callers get the same compiled schema;
/// only the Wasm actor declares the attached store. With an explicit schema
/// function, the key and schema paths resolve inside the generated module.
#[macro_export]
macro_rules! declare_db_store {
    ($vis:vis mod $name:ident, key = $key:literal $(,)?) => {
        /// Retained database allocation and the schema emitted by the app's build script.
        $vis mod $name {
            /// Original stable allocation name.
            pub const DATABASE_KEY: &str = $key;
            /// Compiled application schema for native and Wasm consumers.
            pub const SCHEMA: fn() -> ::ic_db::Schema = compiled;
            include!(concat!(env!("OUT_DIR"), "/schema.rs"));
            #[cfg(target_arch = "wasm32")]
            $crate::declare_db_store!(pub(crate) mod store, key = super::DATABASE_KEY, schema = super::compiled);
        }
    };
    ($vis:vis mod $name:ident, key = $key:expr, schema = $schema:path $(,)?) => {
        $vis mod $name {
            use $crate::db::store::{Database, DefaultMemoryImpl, MemoryRuntime, Store};
            std::thread_local! { static STORE: Store = const { Store::new($key, $schema) }; }
            /// Attach using the actor's existing directory and install-only incarnation.
            pub fn attach(host: &MemoryRuntime<DefaultMemoryImpl>, incarnation: Option<[u8; 16]>) {
                STORE.with(|s| s.attach(host, incarnation));
            }
            /// Borrow the attached database for a synchronous operation.
            pub fn with<R>(op: impl FnOnce(&mut Database) -> R) -> R {
                STORE.with(|s| s.with(op))
            }
        }
    };
}

#[cfg(test)]
mod tests;

/// Declare the generated database and install one Auth-enrolled hub or shard.
/// Put this in the role setup module. IC DB's build script owns schema generation.
/// Native builds need only `db-store`; Wasm builds select `stateful-role`.
#[macro_export]
macro_rules! mount_db_role {
    (hub, database = $key:literal, config = $config:literal, authority = $owner:literal,
     pages = $pages:literal, auth_read = $scope:literal $(,)?) => {
        $crate::declare_db_store!(pub mod database, key = $key);
        #[cfg(target_arch = "wasm32")]
        mod actor {
            use $crate::db::role;
            $crate::mount_auth_lifecycle!(decode_context = $crate::db::role::decode_hub, open = super::open);
            fn open(initial: Option<role::HubConfig>) {
                role::open_hub(initial, $owner, $key, $config, $pages, super::database::store::attach);
            }
            $crate::mount_auth_session!(read = $scope);
            $crate::mount_readiness_endpoint!($crate::db::role::hub_readiness);
        }
    };
    (shard, database = $key:literal, parent = $parent:literal, authority = $owner:literal,
     prefix = $prefix:literal, auth_read = $scope:literal $(,)?) => {
        $crate::declare_db_store!(pub mod database, key = $key);
        #[cfg(target_arch = "wasm32")]
        mod actor {
            use $crate::db::role;
            $crate::mount_auth_lifecycle!(decode_context = $crate::db::role::decode_shard, open = super::open);
            fn open(initial: Option<role::ShardConfig>) {
                role::open_shard(initial, $owner, $prefix, $key, $parent, super::database::store::attach);
            }
            $crate::mount_auth_session!(read = $scope);
            $crate::mount_readiness_endpoint!($crate::db::role::shard_readiness);
        }
    };
}
