//! Full Store custody comparison on both sides of the existing Root upgrade effect.
use super::{invalid, single_upgrade_root};
use crate::{Plan, Result, managed, network::principal, supplier};
use canic_management_protocol::{children::ChildId, stored as wire};

pub(super) fn validate(plan: &Plan) -> Result<()> {
    let Some(store) = plan.maintenance_store() else {
        return Ok(());
    };
    let children = plan.maintenance().ok_or_else(invalid)?;
    let id = principal(&store.store.canister)?;
    if single_upgrade_root(plan)
        || id.to_text() != store.store.canister
        || id == principal("aaaaa-aa")?
        || id == principal("2vxsx-fae")?
        || plan.target() == Some(&store.store.canister)
        || crate::unhex(&store.store.module)?.len() != 32
        || store.store.module != store.store.module.to_ascii_lowercase()
        || (store.operation.is_none() && store.sources.iter().all(Option::is_none))
        || store.sources[children.len()..].iter().any(Option::is_some)
        || store.history.len() >= canic_management_protocol::store_maintenance::MAX_UPGRADES
        || (!store.history.is_empty() && store.operation.is_none())
    {
        return Err(invalid());
    }
    for (index, child) in children.iter().enumerate() {
        let Some(sources) = &store.sources[index] else {
            continue;
        };
        if sources.upgrades.len() != child.upgrades.len() {
            return Err(invalid());
        }
        let slot = ChildId::ALL[index];
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
        if !store.history.is_empty() {
            let chain: Vec<_> = store
                .history
                .iter()
                .chain(std::iter::once(operation))
                .collect();
            for (index, record) in chain.iter().enumerate() {
                if record.approval.attempt.sequence != index as u64 + 1
                    || record.approval.store != store.store.canister
                    || (index < store.history.len() && !crate::store_upgrade::ready(record))
                    || (index > 0
                        && chain[index - 1].approval.module_hash != record.approval.previous_module)
                    || record.approval.attempt.operation == [0; 32]
                    || chain[..index].iter().any(|old| {
                        old.approval.attempt.operation == record.approval.attempt.operation
                    })
                {
                    return Err(invalid());
                }
            }
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
        ic_wire::encode(
            &canic_management_protocol::custody::Root {
                stored: expected(plan)?,
                store_operation: Some(operation.clone()),
            },
            ic_wire::Encoding::Payload,
            canic_management_protocol::custody::ROOT_LIMITS,
        )
        .map_err(supplier)?;
        if !store.history.is_empty() {
            ic_wire::encode(
                &canic_management_protocol::custody::RootHistory {
                    current: canic_management_protocol::custody::Root {
                        stored: expected(plan)?,
                        store_operation: Some(operation.clone()),
                    },
                    store_history: store.history.clone(),
                },
                ic_wire::Encoding::Payload,
                canic_management_protocol::custody::ROOT_HISTORY_LIMITS,
            )
            .map_err(supplier)?;
        }
    }
    ic_wire::encode(&expected(plan)?, ic_wire::Encoding::Payload, wire::LIMITS)
        .map_err(supplier)?;
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

fn expected(plan: &Plan) -> Result<wire::Custody> {
    let children = plan.maintenance().ok_or_else(invalid)?;
    Ok(wire::Custody {
        children: std::array::from_fn(|i| {
            children.get(i).cloned().unwrap_or_else(|| {
                canic_management_protocol::custody::Custody {
                    creation: canic_management_protocol::Status::default(),
                    upgrades: Vec::new(),
                }
            })
        }),
        sources: plan
            .maintenance_store()
            .ok_or_else(invalid)?
            .sources
            .clone(),
    })
}
pub(super) fn matches(plan: &Plan, method: &str, bytes: &[u8]) -> Result<bool> {
    if method != super::observations(plan, false)[0].1 {
        return Err(invalid());
    }
    let contract = match method {
        "observe_root_history" => canic_management_protocol::custody::ROOT_HISTORY,
        "observe_root_custody" => canic_management_protocol::custody::ROOT,
        "observe_stored_children" => wire::CUSTODY,
        _ => return Err(invalid()),
    };
    let actual = crate::root_custody::decode(contract, bytes)?;
    let store = plan.maintenance_store().ok_or_else(invalid)?;
    Ok(
        actual.children == crate::root_custody::children(expected(plan)?)?
            && actual.store_operation == store.operation
            && actual.store_history == store.history,
    )
}
