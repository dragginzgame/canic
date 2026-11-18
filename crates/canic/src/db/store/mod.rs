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
/// The key and schema paths resolve inside the generated module.
#[macro_export]
macro_rules! declare_db_store {
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
