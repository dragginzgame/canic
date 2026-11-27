//! Decode the selected Root custody observation and preserve service identities.
//! This projection grants no authority and never probes or falls back to another endpoint.

#[cfg(test)]
mod tests;

use crate::{Error, Result, supplier};
use canic_management_protocol::{application, children::ChildId};
#[cfg(test)]
use canic_management_protocol::{custody, store_maintenance, stored};

pub(crate) fn child(
    value: &application::Custody,
    slot: ChildId,
) -> Result<&application::ChildCustody> {
    slot.service_index()
        .and_then(|index| value.children.get(index))
        .filter(|child| child.slot == slot)
        .ok_or_else(|| Error::Invalid("Root custody omitted the selected child identity".into()))
}

#[cfg(test)]
pub(crate) fn fixture(
    root: custody::RootHistory,
    applications: Vec<Option<application::Lifecycle>>,
) -> application::Custody {
    let stored = root.current.stored;
    assert_eq!(stored.children.len(), stored.sources.len());
    assert_eq!(stored.children.len(), applications.len());
    let children = stored
        .children
        .into_iter()
        .zip(stored.sources)
        .zip(applications)
        .enumerate()
        .map(
            |(i, ((journal, sources), lifecycle))| application::ChildCustody {
                slot: ChildId::service(i).unwrap(),
                journal,
                sources,
                lifecycle,
            },
        )
        .collect();
    application::Custody {
        children,
        store_operation: root.current.store_operation,
        store_history: root.store_history,
    }
}

#[cfg(test)]
pub(crate) fn frame(
    current: custody::Root,
    store_history: Vec<store_maintenance::Status>,
) -> Vec<u8> {
    use canic_management_protocol::application;
    let empty = vec![None; current.stored.children.len()];
    let value = fixture(
        custody::RootHistory {
            current,
            store_history,
        },
        empty,
    );
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
    if value.children.len() < ChildId::HISTORICAL_SERVICES.len()
        || value
            .children
            .iter()
            .enumerate()
            .any(|(i, child)| ChildId::service(i) != Some(child.slot))
    {
        return Err(Error::Invalid(
            "Root custody has inconsistent service slots".into(),
        ));
    }
    Ok(value)
}
