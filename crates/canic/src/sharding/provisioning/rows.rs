//! Optional bounded reads of application-owned IC DB rows.
use super::AuthSubject;
use crate::auth::session::Authorization;
use ic_db::{DatabaseSession, Entity};

/// Read the provisioning row for a session already authorized by the endpoint.
/// Uses the same unique account index and one-row byte budget as provisioning.
/// # Panics
/// Traps on conflicting database access, an incompatible index, or an oversized row.
#[must_use]
pub fn session_row<E: Entity>(database: &DatabaseSession, view: &Authorization) -> Option<E> {
    account_row(database, AuthSubject::from(view))
}

/// Optional IC DB lookup for applications with a unique issuer/subject index.
/// # Panics
/// Refuses an incompatible index, oversized row or conflicting access.
#[must_use]
pub fn account_row<E: Entity>(database: &DatabaseSession, account: AuthSubject) -> Option<E> {
    let db = database.read().expect("read provisioned account");
    assert!(
        E::SCHEMA.indexes.iter().any(|index| {
            index.unique && index.name == "issuer,subject" && index.fields == ["issuer", "subject"]
        }),
        "provisioning requires a unique issuer,subject index"
    );
    let row = db
        .index_prefix::<E>(
            "issuer,subject",
            &[account.issuer.into(), account.subject.into()],
            1,
        )
        .expect("lookup provisioned account")
        .pop()?;
    let encoded = row.encode().expect("encode provisioned account");
    assert!(encoded.len() <= 1024, "provisioning row exceeds 1 KiB");

    Some(row)
}

impl From<&Authorization> for AuthSubject {
    fn from(view: &Authorization) -> Self {
        Self {
            issuer: view.issuer(),
            subject: view.subject(),
        }
    }
}
