//! Optional bounded reads of application-owned IC DB rows.
use super::AuthSubject;
use crate::auth::session::Authorization;
use ic_db::{Entity, QuerySession};

/// Optional IC DB lookup for applications with a unique issuer/subject index.
/// # Panics
/// Refuses an incompatible index, oversized row or conflicting access.
#[must_use]
pub fn account_row<E: Entity>(database: &impl QuerySession<E>, account: AuthSubject) -> Option<E> {
    assert!(
        E::SCHEMA.indexes.iter().any(|index| {
            index.unique && index.name == "issuer,subject" && index.fields == ["issuer", "subject"]
        }),
        "provisioning requires a unique issuer,subject index"
    );
    let row = database
        .entity_query()
        .using_index(
            "issuer,subject",
            &[account.issuer.into(), account.subject.into()],
        )
        .work_limit(1)
        .first()
        .expect("lookup provisioned account")?;
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
