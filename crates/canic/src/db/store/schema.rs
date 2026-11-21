//! Attach a schema owner's complete store roster to an admitted Storage directory.

use ic_db::{CanisterSession, CompiledCanister, Database, DatabaseSession, Principal};
use ic_storage::{DefaultMemoryImpl, MemoryRuntime, StableKey};
use sha2::{Digest, Sha256};

fn incarnation(key: &str, nonce: [u8; 16], owner: Principal) -> [u8; 16] {
    let digest = Sha256::new()
        .chain_update(key)
        .chain_update(nonce)
        .chain_update(owner.as_slice())
        .finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&digest[..16]);
    id
}

/// Attach the compiled roster to an admitted directory. On install, supply the
/// approved nonce and this canister's principal. On upgrade, supply `None`.
/// # Panics
/// Traps if any lease or database fails admission.
#[must_use]
pub fn attach_schema<C: CompiledCanister>(
    host: &MemoryRuntime<DefaultMemoryImpl>,
    install: Option<([u8; 16], Principal)>,
) -> CanisterSession<C> {
    CanisterSession::try_attach(|store| {
        let key = StableKey::parse(store.key).expect("schema storage key");
        let memory = host.open_memory(&key).expect("schema allocation");
        let schema = (store.schema)();
        match install {
            Some((nonce, owner)) => {
                Database::create(memory, schema, incarnation(store.key, nonce, owner))
            }
            None => Database::open(memory, schema),
        }
        .map(DatabaseSession::new)
    })
    .expect("attach compiled canister stores")
}

/// Select all databases owned by an IC DB canister. The application's lifecycle
/// supplies the admitted directory and install identity to `database::attach`.
#[macro_export]
macro_rules! mount_db {
    (schema = $schema:path $(,)?) => {
        type __CanicDatabaseOwner = $schema;
        /// Databases declared by this canister schema.
        pub mod database {
            use super::__CanicDatabaseOwner as Owner;
            use ::ic_db::{CanisterSession, CompiledCanister, Principal};
            use $crate::db::store::{DefaultMemoryImpl, MemoryRuntime};
            $crate::__canic_report_mount!("canic:module:db");

            /// Stable keys to include in the application's Storage bootstrap.
            pub fn keys() -> Vec<&'static str> {
                Owner::STORES.iter().map(|store| store.key).collect()
            }

            /// Create on install or strictly reopen every store on upgrade.
            pub fn attach(host: &MemoryRuntime<DefaultMemoryImpl>, install: Option<([u8; 16], Principal)>) {
                let session = $crate::db::store::attach_schema::<Owner>(host, install);
                store::SESSION.with(|slot| assert!(slot.set(session).is_ok(), "duplicate database attachment"));
            }

            pub(crate) mod store {
                use super::{CanisterSession, Owner};
                std::thread_local! {
                    pub(super) static SESSION: std::cell::OnceCell<CanisterSession<Owner>> = const { std::cell::OnceCell::new() };
                }
                pub(crate) fn session() -> CanisterSession<Owner> {
                    SESSION.with(|slot| slot.get().expect("database lifecycle has not run").clone())
                }
            }
        }
    };
}
