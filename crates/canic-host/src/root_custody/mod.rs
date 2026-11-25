//! Decode the selected Root custody observation and preserve service identities.
//! This projection grants no authority and never probes or falls back to another endpoint.

#[cfg(test)]
mod tests;

use crate::{Error, Result, supplier};
use canic_management_protocol::{
    application, children::ChildId, custody, store_maintenance, stored,
};
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

#[cfg(test)]
pub(crate) fn frame(
    current: custody::Root,
    store_history: Vec<store_maintenance::Status>,
) -> Vec<u8> {
    use canic_management_protocol::application;
    let value = application::Custody {
        applications: vec![None; current.stored.children.len()],
        root: custody::RootHistory {
            current,
            store_history,
        },
    };
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: application::CUSTODY,
            direction: ic_wire::Direction::Response,
        },
        application::LIMITS,
    )
    .unwrap()
}

pub(crate) fn decode(method: u64, bytes: &[u8]) -> Result<application::Custody> {
    if !matches!(method, application::CUSTODY | application::CHILD_EVIDENCE) {
        return Err(Error::Invalid(
            "unsupported Root custody observation".into(),
        ));
    }
    let value: application::Custody = ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method,
            direction: ic_wire::Direction::Response,
        },
        application::LIMITS,
    )
    .map_err(supplier)?;
    let stored = &value.root.current.stored;
    if stored.children.len() < ChildId::HISTORICAL_SERVICES.len()
        || value.applications.len() != stored.children.len()
        || stored.sources.len() != stored.children.len()
    {
        return Err(Error::Invalid(
            "Root custody has inconsistent service slots".into(),
        ));
    }
    Ok(value)
}
