//! Full Store custody comparison on both sides of the existing Root upgrade effect.
use super::invalid;
use crate::{Plan, Result, managed, network::principal};
use canic_management_protocol::{application, children::ChildId, stored as wire};

pub(super) fn validate(plan: &Plan) -> Result<()> {
    let store = plan.maintenance_store().ok_or_else(invalid)?;
    let children = plan.maintenance().ok_or_else(invalid)?;
    let id = principal(&store.store.canister)?;
    if id.to_text() != store.store.canister
        || id == principal("aaaaa-aa")?
        || id == principal("2vxsx-fae")?
        || plan.target() == Some(&store.store.canister)
        || crate::unhex(&store.store.module)?.len() != 32
        || store.store.module != store.store.module.to_ascii_lowercase()
        || store.sources.len() != children.len()
        || (!store.history.is_empty() && store.operation.is_none())
    {
        return Err(invalid());
    }
    for (index, child) in children.iter().enumerate() {
        let sources = store.sources[index].as_ref().ok_or_else(invalid)?;
        if sources.upgrades.len() != child.upgrades.len() {
            return Err(invalid());
        }
        let slot = ChildId::service(index).ok_or_else(invalid)?;
        let approval = child.creation.approval.as_ref().ok_or_else(invalid)?;
        delivery(
            &store.store.canister,
            slot,
            wire::STAGE,
            approval,
            approval.module_hash,
            &sources.creation,
        )?;
        for (status, source) in child.upgrades.iter().zip(&sources.upgrades) {
            let approval = status.approval.as_ref().ok_or_else(invalid)?;
            delivery(
                &store.store.canister,
                slot,
                wire::STAGE_UPGRADE,
                approval,
                approval.module_hash,
                source,
            )?;
        }
    }
    if let Some(operation) = &store.operation {
        let mut operations = std::collections::BTreeSet::new();
        let mut previous_module = None;
        for (index, record) in store
            .history
            .iter()
            .chain(std::iter::once(operation))
            .enumerate()
        {
            let approval = &record.approval;
            if approval.attempt.sequence != index as u64 + 1
                || approval.store != store.store.canister
                || (index < store.history.len() && !crate::store_upgrade::ready(record))
                || previous_module.is_some_and(|hash| hash != approval.previous_module)
                || approval.attempt.operation == [0; 32]
                || !operations.insert(approval.attempt.operation)
            {
                return Err(invalid());
            }
            previous_module = Some(approval.module_hash);
        }
        let approval = &operation.approval;
        let operator = principal(&operation.operator)?;
        if approval.store != store.store.canister
            || operator.to_text() != operation.operator
            || operator == principal("2vxsx-fae")?
            || approval.attempt.sequence == 0
            || approval.attempt.operation == [0; 32]
            || approval.previous_module == approval.module_hash
            || approval.previous_module == [0; 32]
            || approval.module_hash == [0; 32]
            || ![
                crate::hex(&approval.previous_module),
                crate::hex(&approval.module_hash),
            ]
            .contains(&store.store.module)
        {
            return Err(invalid());
        }
    }
    Ok(())
}

fn delivery<A: serde::Serialize>(
    store: &str,
    slot: ChildId,
    method: u64,
    approval: &A,
    hash: [u8; 32],
    source: &wire::Delivery,
) -> Result<()> {
    if source.reference.store != store
        || !source.reference.manifest.valid()
        || source.reference.manifest.hash != hash
        || source.cache_deadline.is_none()
    {
        return Err(invalid());
    }
    managed::stored::delivery(
        slot,
        &wire::Stage {
            approval,
            reference: source.reference.clone(),
        },
        method,
        source,
        true,
    )
}

pub(super) fn expected(plan: &Plan) -> Result<application::Custody> {
    let journals = plan.maintenance().ok_or_else(invalid)?;
    let store = plan.maintenance_store().ok_or_else(invalid)?;
    let children = store
        .sources
        .iter()
        .enumerate()
        .map(|(i, sources)| {
            Ok(application::ChildCustody {
                slot: ChildId::service(i).ok_or_else(invalid)?,
                journal: journals.get(i).ok_or_else(invalid)?.clone(),
                sources: sources.clone(),
                lifecycle: None,
            })
        })
        .collect::<Result<_>>()?;
    Ok(application::Custody {
        children,
        store_operation: store.operation.clone(),
        store_history: store.history.clone(),
    })
}
