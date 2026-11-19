//! Install or reopen one typed database after the actor admits its complete Storage directory.

#[cfg(test)]
mod tests;

use ic_db::Schema;
use ic_storage::StableKey;
use std::cell::RefCell;

#[doc(hidden)]
pub use ic_db::{Database, DatabaseSession};
#[doc(hidden)]
pub use ic_storage::{DefaultMemoryImpl, MemoryRuntime};

/// Declare in actor-local state. The key and schema must match the retained database.
pub struct Store {
    key: &'static str,
    schema: fn() -> Schema,
    database: RefCell<Option<Attachment>>,
}

// Both forms own the same lease. Keep legacy calls allocation-free until a session is requested.
enum Attachment {
    Exclusive(Database),
    Shared(DatabaseSession),
}

impl Store {
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
    pub fn attach(&self, host: &MemoryRuntime<DefaultMemoryImpl>, incarnation: Option<[u8; 16]>) {
        let mut slot = self.database.borrow_mut();
        assert!(slot.is_none(), "database already attached: {}", self.key);
        let key = StableKey::parse(self.key).expect("database allocation key");
        let memory = host.open_memory(&key).expect("database allocation");
        let schema = (self.schema)();
        *slot = Some(Attachment::Exclusive(match incarnation {
            Some(id) => Database::create(memory, schema, id).expect("create declared database"),
            None => Database::open(memory, schema).expect("reopen declared database"),
        }));
    }

    /// Obtain a handle without borrowing database contents.
    /// # Panics
    /// Traps before lifecycle attachment or during an exclusive store callback.
    #[must_use]
    pub fn session(&self) -> DatabaseSession {
        let mut slot = self
            .database
            .try_borrow_mut()
            .expect("database is in use by an exclusive store callback");
        if let Some(Attachment::Shared(session)) = slot.as_ref() {
            return session.clone();
        }
        let Some(Attachment::Exclusive(db)) = slot.take() else {
            panic!("database lifecycle has not run: {}", self.key);
        };
        let session = DatabaseSession::new(db);
        *slot = Some(Attachment::Shared(session.clone()));
        session
    }

    /// Borrow the attached database for one synchronous application operation.
    ///
    /// # Panics
    /// Traps before attachment, during nested legacy access or while session handles are live.
    pub fn with<R>(&self, operation: impl FnOnce(&mut Database) -> R) -> R {
        let mut slot = self.database.borrow_mut();
        if let Some(Attachment::Shared(session)) =
            slot.take_if(|attachment| matches!(attachment, Attachment::Shared(_)))
        {
            *slot = Some(match session.try_into_database() {
                Ok(db) => Attachment::Exclusive(db),
                Err(session) => Attachment::Shared(session),
            });
        }
        match slot.as_mut() {
            Some(Attachment::Exclusive(db)) => operation(db),
            Some(Attachment::Shared(_)) => panic!("drop database sessions before legacy access"),
            None => panic!("database lifecycle has not run"),
        }
    }
}

/// Declare an actor-local database module with `attach` and synchronous `with`.
/// Supply only the allocation key to bind IC DB's generated `OUT_DIR/schema.rs`
/// from the consuming package. All targets use the same compiled schema
/// and explicitly attached store. With an explicit schema
/// function, the key and schema paths resolve inside the generated module.
#[macro_export]
macro_rules! declare_db_store {
    ($vis:vis mod $name:ident, key = $key:literal $(,)?) => {
        $crate::declare_db_store!(@generated $vis mod $name, $key, {});
    };
    (@generated $vis:vis mod $name:ident, $key:literal, { $($role:item)* }) => {
        /// Retained database allocation and the schema emitted by the app's build script.
        $vis mod $name {
            /// Original stable allocation name.
            pub const DATABASE_KEY: &str = $key;
            /// Compiled application schema for native and Wasm consumers.
            pub const SCHEMA: fn() -> ::ic_db::Schema = compiled;

            $($role)*

            include!(concat!(env!("OUT_DIR"), "/schema.rs"));

            $crate::declare_db_store!(pub(crate) mod store, key = super::DATABASE_KEY, schema = super::compiled);
        }
    };
    ($vis:vis mod $name:ident, key = $key:expr, schema = $schema:path $(,)?) => {
        $vis mod $name {
            use $crate::db::store::{Database, DatabaseSession, DefaultMemoryImpl, MemoryRuntime, Store};

            $crate::__canic_report_mount!("canic:module:db");

            std::thread_local! { static STORE: Store = const { Store::new($key, $schema) }; }

            /// Attach using the actor's existing directory and install-only incarnation.
            pub fn attach(host: &MemoryRuntime<DefaultMemoryImpl>, incarnation: Option<[u8; 16]>) {
                STORE.with(|s| s.attach(host, incarnation));
            }

            /// Obtain a session for this attached database.
            /// # Panics
            /// Traps before attachment or during an exclusive store callback.
            #[must_use]
            #[allow(dead_code, reason = "Mounted consumers may use only the exclusive API")]
            pub fn session() -> DatabaseSession {
                STORE.with(Store::session)
            }

            /// Borrow the attached database for a synchronous operation.
            #[allow(dead_code, reason = "Mounted consumers may use only the session API")]
            pub fn with<R>(op: impl FnOnce(&mut Database) -> R) -> R {
                STORE.with(|s| s.with(op))
            }
        }
    };
}

/// Declare the generated database and install one Auth-enrolled hub or shard.
/// Put this in the role setup module. IC DB's build script owns schema generation.
/// Select `stateful-role` for the mounted actor on both targets; plain stores use `db-store`.
/// A shard retains its approved parent in `<prefix>hub.v1`. For a predecessor
/// using a different allocation, specify its exact `parent_storage_key`.
#[macro_export]
macro_rules! mount_db_role {
    (hub, database = $key:literal, config = $config:literal, authority = $owner:literal,
     $(prefix = $prefix:literal,)? pages = $pages:literal, auth_read = $scope:literal $(,)?) => {
        $crate::declare_db_store!(@generated pub mod database, $key, {
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
    (shard, database = $key:literal, authority = $owner:literal,
     prefix = $prefix:literal, $(pages = $pages:literal,)? auth_read = $scope:literal $(,)?) => {
        $crate::mount_db_role!(
            shard,
            database = $key,
            parent_storage_key = concat!($prefix, "hub.v1"),
            authority = $owner,
            prefix = $prefix,
            $(pages = $pages,)?
            auth_read = $scope
        );
    };
    (shard, database = $key:literal, parent_storage_key = $parent:expr, authority = $owner:literal,
     prefix = $prefix:literal, $(pages = $pages:literal,)? auth_read = $scope:literal $(,)?) => {
        $crate::declare_db_store!(@generated pub mod database, $key, {
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
        use $crate::db::role;

        $crate::mount_auth_lifecycle!(
            decode_context = $crate::db::role::$decode,
            open = super::open,
        );

        fn open($initial: Option<role::$config>) $open

        $crate::mount_auth_session!(read = super::AUTH_READ);
        $crate::mount_readiness_endpoint!($crate::db::role::$ready);
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

/// Obtain a session from the single role mount. Import the mounted `database`
/// module alongside this macro in an application operations module.
#[macro_export]
macro_rules! db {
    () => {
        database::store::session()
    };
}
