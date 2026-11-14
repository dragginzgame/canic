//! Local revocation before any effect preparation, under the existing journal lock.
use crate::{Error, Result, store, supplier};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Retained revocation of an unsubmitted whole-fleet approval.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Cancellation {
    /// Original approval, whose artifacts and intent remain retained.
    pub operation: String,
    /// Local cancellation time, in Unix nanoseconds.
    pub cancelled_at_ns: u64,
}

fn pristine(dir: &Path) -> Result<()> {
    for (leaf, expected) in [
        ("root", &["module.wasm", "plan.json"][..]),
        ("child", &["module.wasm"][..]),
    ] {
        for entry in std::fs::read_dir(dir.join(leaf)).map_err(supplier)? {
            let entry = entry.map_err(supplier)?;
            if !entry.file_type().map_err(supplier)?.is_file()
                || !expected.iter().any(|name| entry.file_name() == *name)
            {
                return Err(Error::Blocked(
                    "effect preparation already began; preserve this operation and resume or inspect it, do not replace it".into(),
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn read(dir: &Path, id: &str) -> Result<Option<Cancellation>> {
    let path = dir.join("cancelled.json");
    if !path.try_exists().map_err(supplier)? {
        return Ok(None);
    }
    let record: Cancellation = store::json(&path)?;
    if record.operation != id {
        return Err(Error::Invalid(
            "cancellation does not match its approval".into(),
        ));
    }
    pristine(dir)?;
    Ok(Some(record))
}

/// Cancel an unsubmitted whole-fleet approval, retaining all evidence.
/// Idempotent; after cancellation the same digest can never apply.
/// # Errors
/// Refuses single-canister approvals, corruption, concurrent writers or any effect preparation.
pub fn cancel_fleet_plan(journal: &Path, id: &str) -> Result<Cancellation> {
    let store = store::Store::open(journal)?;
    let dir = store.operation(id)?;
    super::load(&dir, id)?;
    if let Some(record) = read(&dir, id)? {
        return Ok(record);
    }
    pristine(&dir)?;
    let record = Cancellation {
        operation: id.into(),
        cancelled_at_ns: crate::now()?,
    };
    store::put_json(&dir.join("cancelled.json"), &record)?;
    Ok(record)
}
