//! Install or reopen one typed database after the actor admits its complete Storage directory.

mod schema;
#[doc(hidden)]
pub use schema::attach_schema;

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
/// Select an IC DB store from the schema owner. A custom lifecycle can instead
/// provide an explicit key and schema function inside its generated module.
#[macro_export]
macro_rules! declare_db_store {
    ($vis:vis mod $name:ident, store = $store:path $(,)?) => {
        /// Database allocation selected from the application schema.
        $vis mod $name {
            use $store as Definition;
            /// Stable allocation key declared by IC DB.
            pub const DATABASE_KEY: &str = <Definition as ::ic_db::Store>::KEY;
            /// Compiled schema for this allocation.
            pub const SCHEMA: fn() -> ::ic_db::Schema = Definition::compiled;

            $crate::declare_db_store!(pub(crate) mod store, key = super::DATABASE_KEY, schema = super::SCHEMA);
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

/// Obtain the attached session. In a schema-mounted role, IC DB selects the
/// store by entity type. Import the mounted `database` module beside this macro.
#[macro_export]
macro_rules! db {
    () => {
        database::store::session()
    };
}
