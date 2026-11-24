//! Decode an explicitly selected historical contract into identity-keyed Host evidence.
//! This projection grants no authority and never probes or falls back to another endpoint.

#[cfg(test)]
mod tests;

use crate::{Error, Result, supplier};
use canic_management_protocol::{children::ChildId, custody, store_maintenance, stored};
use std::collections::BTreeMap;

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Child {
    pub journal: custody::Custody,
    pub sources: Option<stored::Sources>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Snapshot {
    pub children: BTreeMap<ChildId, Child>,
    pub store_operation: Option<store_maintenance::Status>,
    pub store_history: Vec<store_maintenance::Status>,
}

impl Snapshot {
    pub fn child(&self, id: ChildId) -> Result<&Child> {
        self.children.get(&id).ok_or_else(|| {
            Error::Invalid("Root custody omitted the selected child identity".into())
        })
    }
}

// Historical array positions become explicit identities only at the codec boundary.
// Empty slots remain present; they are evidence, not missing observations.
pub(crate) fn children(value: stored::Custody) -> Result<BTreeMap<ChildId, Child>> {
    if value.children.len() < ChildId::HISTORICAL_SERVICES.len()
        || value.children.len() != value.sources.len()
    {
        return Err(Error::Invalid(
            "historical Root custody cannot represent the admitted child identities".into(),
        ));
    }
    Ok((0..value.children.len())
        .map(|index| ChildId::service(index).expect("bounded custody"))
        .zip(value.children.into_iter().zip(value.sources))
        .map(|(id, (journal, sources))| (id, Child { journal, sources }))
        .collect())
}

pub(crate) fn decode(method: u64, bytes: &[u8]) -> Result<Snapshot> {
    let encoding = ic_wire::Encoding::Frame {
        method,
        direction: ic_wire::Direction::Response,
    };
    let (current, store_history) = match method {
        stored::CUSTODY => {
            let value = ic_wire::decode(bytes, encoding, stored::LIMITS).map_err(supplier)?;
            (
                custody::Root {
                    stored: value,
                    store_operation: None,
                },
                Vec::new(),
            )
        }
        custody::ROOT => (
            ic_wire::decode(bytes, encoding, custody::ROOT_LIMITS).map_err(supplier)?,
            Vec::new(),
        ),
        custody::ROOT_HISTORY => {
            let value: custody::RootHistory =
                ic_wire::decode(bytes, encoding, custody::ROOT_HISTORY_LIMITS).map_err(supplier)?;
            (value.current, value.store_history)
        }
        _ => {
            return Err(Error::Invalid(
                "unsupported complete Root custody contract".into(),
            ));
        }
    };
    Ok(Snapshot {
        children: children(current.stored)?,
        store_operation: current.store_operation,
        store_history,
    })
}
