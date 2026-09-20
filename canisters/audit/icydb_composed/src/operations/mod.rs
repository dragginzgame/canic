//! Reachable audit operations; disabled subjects retain the same wire adapters.

use candid::CandidType;
#[cfg(feature = "binding")]
use canic_composed_empty_schema::*;
#[cfg(feature = "write")]
use icydb::db::{TypedWriteAdapter, WriteCell};
#[cfg(feature = "query")]
use icydb::{db::DynamicQuery, traits::EntitySource};
#[cfg(feature = "binding")]
use icydb::{
    db::{DbSession, TypedEntityAdapter},
    traits::CanisterKind,
};

/// Observable audit outcomes, shared unchanged by every measurement subject.
#[derive(CandidType)]
#[expect(
    dead_code,
    reason = "each subject constructs only its supported audit outcomes"
)]
pub(crate) enum AuditOutcome {
    Unavailable,
    InvalidEntity,
    DatabaseUnavailable,
    Rejected,
    Binding,
    Rows(u32),
    Inserted,
}

/// Execute the selected binding subject under IcyDB's request owner.
#[cfg(feature = "binding")]
pub(crate) fn binding(entity: u8) -> AuditOutcome {
    icydb::db::with_request_execution(|| {
        let Ok(database) = crate::db() else {
            return AuditOutcome::DatabaseUnavailable;
        };
        match entity {
            0 => typed_binding::<_, AuditEntity01>(&database),
            #[cfg(feature = "ten")]
            1 => typed_binding::<_, AuditEntity02>(&database),
            #[cfg(feature = "ten")]
            2 => typed_binding::<_, AuditEntity03>(&database),
            #[cfg(feature = "ten")]
            3 => typed_binding::<_, AuditEntity04>(&database),
            #[cfg(feature = "ten")]
            4 => typed_binding::<_, AuditEntity05>(&database),
            #[cfg(feature = "ten")]
            5 => typed_binding::<_, AuditEntity06>(&database),
            #[cfg(feature = "ten")]
            6 => typed_binding::<_, AuditEntity07>(&database),
            #[cfg(feature = "ten")]
            7 => typed_binding::<_, AuditEntity08>(&database),
            #[cfg(feature = "ten")]
            8 => typed_binding::<_, AuditEntity09>(&database),
            #[cfg(feature = "ten")]
            9 => typed_binding::<_, AuditEntity10>(&database),
            _ => AuditOutcome::InvalidEntity,
        }
    })
}

/// Keep the same endpoint in subjects where binding is unavailable.
#[cfg(not(feature = "binding"))]
pub(crate) const fn binding(_entity: u8) -> AuditOutcome {
    AuditOutcome::Unavailable
}

/// Execute the selected page subject under IcyDB's request owner.
#[cfg(feature = "query")]
pub(crate) fn page(entity: u8) -> AuditOutcome {
    icydb::db::with_request_execution(|| {
        let Ok(database) = crate::db() else {
            return AuditOutcome::DatabaseUnavailable;
        };
        match entity {
            0 => typed_page::<_, AuditEntity01>(&database),
            #[cfg(feature = "ten")]
            1 => typed_page::<_, AuditEntity02>(&database),
            #[cfg(feature = "ten")]
            2 => typed_page::<_, AuditEntity03>(&database),
            #[cfg(feature = "ten")]
            3 => typed_page::<_, AuditEntity04>(&database),
            #[cfg(feature = "ten")]
            4 => typed_page::<_, AuditEntity05>(&database),
            #[cfg(feature = "ten")]
            5 => typed_page::<_, AuditEntity06>(&database),
            #[cfg(feature = "ten")]
            6 => typed_page::<_, AuditEntity07>(&database),
            #[cfg(feature = "ten")]
            7 => typed_page::<_, AuditEntity08>(&database),
            #[cfg(feature = "ten")]
            8 => typed_page::<_, AuditEntity09>(&database),
            #[cfg(feature = "ten")]
            9 => typed_page::<_, AuditEntity10>(&database),
            _ => AuditOutcome::InvalidEntity,
        }
    })
}

/// Keep the same endpoint in subjects where page is unavailable.
#[cfg(not(feature = "query"))]
pub(crate) const fn page(_entity: u8) -> AuditOutcome {
    AuditOutcome::Unavailable
}

/// Execute the selected insert subject under IcyDB's request owner.
#[cfg(feature = "write")]
pub(crate) fn insert(entity: u8, id: u64, value: u64) -> AuditOutcome {
    icydb::db::with_request_execution(|| {
        let Ok(database) = crate::db() else {
            return AuditOutcome::DatabaseUnavailable;
        };
        match entity {
            0 => typed_insert::<_, AuditEntity01, _>(
                &database,
                AuditEntity01Insert {
                    id: WriteCell::Value(icydb::types::Id::from_key(id)),
                    value: WriteCell::Value(value),
                },
            ),
            #[cfg(feature = "ten")]
            1 => typed_insert::<_, AuditEntity02, _>(
                &database,
                AuditEntity02Insert {
                    id: WriteCell::Value(icydb::types::Id::from_key(id)),
                    value: WriteCell::Value(value),
                },
            ),
            #[cfg(feature = "ten")]
            2 => typed_insert::<_, AuditEntity03, _>(
                &database,
                AuditEntity03Insert {
                    id: WriteCell::Value(icydb::types::Id::from_key(id)),
                    value: WriteCell::Value(value),
                },
            ),
            #[cfg(feature = "ten")]
            3 => typed_insert::<_, AuditEntity04, _>(
                &database,
                AuditEntity04Insert {
                    id: WriteCell::Value(icydb::types::Id::from_key(id)),
                    value: WriteCell::Value(value),
                },
            ),
            #[cfg(feature = "ten")]
            4 => typed_insert::<_, AuditEntity05, _>(
                &database,
                AuditEntity05Insert {
                    id: WriteCell::Value(icydb::types::Id::from_key(id)),
                    value: WriteCell::Value(value),
                },
            ),
            #[cfg(feature = "ten")]
            5 => typed_insert::<_, AuditEntity06, _>(
                &database,
                AuditEntity06Insert {
                    id: WriteCell::Value(icydb::types::Id::from_key(id)),
                    value: WriteCell::Value(value),
                },
            ),
            #[cfg(feature = "ten")]
            6 => typed_insert::<_, AuditEntity07, _>(
                &database,
                AuditEntity07Insert {
                    id: WriteCell::Value(icydb::types::Id::from_key(id)),
                    value: WriteCell::Value(value),
                },
            ),
            #[cfg(feature = "ten")]
            7 => typed_insert::<_, AuditEntity08, _>(
                &database,
                AuditEntity08Insert {
                    id: WriteCell::Value(icydb::types::Id::from_key(id)),
                    value: WriteCell::Value(value),
                },
            ),
            #[cfg(feature = "ten")]
            8 => typed_insert::<_, AuditEntity09, _>(
                &database,
                AuditEntity09Insert {
                    id: WriteCell::Value(icydb::types::Id::from_key(id)),
                    value: WriteCell::Value(value),
                },
            ),
            #[cfg(feature = "ten")]
            9 => typed_insert::<_, AuditEntity10, _>(
                &database,
                AuditEntity10Insert {
                    id: WriteCell::Value(icydb::types::Id::from_key(id)),
                    value: WriteCell::Value(value),
                },
            ),
            _ => AuditOutcome::InvalidEntity,
        }
    })
}

/// Keep the same endpoint in subjects where insert is unavailable.
#[cfg(not(feature = "write"))]
pub(crate) const fn insert(_entity: u8, _id: u64, _value: u64) -> AuditOutcome {
    AuditOutcome::Unavailable
}

#[cfg(feature = "binding")]
fn typed_binding<C: CanisterKind, E: TypedEntityAdapter>(database: &DbSession<C>) -> AuditOutcome {
    match E::typed_binding(database) {
        Ok(_) => AuditOutcome::Binding,
        Err(_) => AuditOutcome::Rejected,
    }
}

#[cfg(feature = "query")]
fn typed_page<C: CanisterKind, E: EntitySource + TypedEntityAdapter>(
    database: &DbSession<C>,
) -> AuditOutcome {
    let Ok(binding) = E::typed_binding(database) else {
        return AuditOutcome::Rejected;
    };
    let request = DynamicQuery::new(E::ENTITY).limit(16);
    let mut cursor = database.prepare_live_page_cursor(binding, request);
    let Ok(Some(rows)) = cursor.next_trusted_page() else {
        return AuditOutcome::Rejected;
    };
    let Ok(count) = u32::try_from(rows.len()) else {
        return AuditOutcome::Rejected;
    };
    for row in rows {
        if E::decode_row(cursor.binding(), row).is_err() {
            return AuditOutcome::Rejected;
        }
    }
    AuditOutcome::Rows(count)
}

#[cfg(feature = "write")]
fn typed_insert<C, E, W>(database: &DbSession<C>, input: W) -> AuditOutcome
where
    C: CanisterKind,
    E: TypedEntityAdapter,
    W: TypedWriteAdapter<Entity = E>,
{
    let Ok(binding) = E::typed_binding(database) else {
        return AuditOutcome::Rejected;
    };
    let Ok(write) = input.encode_write(&binding) else {
        return AuditOutcome::Rejected;
    };
    let Ok(row) = database.execute_trusted_typed_write_row(write) else {
        return AuditOutcome::Rejected;
    };
    match E::decode_row(&binding, row) {
        Ok(_) => AuditOutcome::Inserted,
        Err(_) => AuditOutcome::Rejected,
    }
}
