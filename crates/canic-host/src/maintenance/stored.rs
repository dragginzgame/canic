//! Full Store custody comparison on both sides of the existing Root upgrade effect.
use super::{invalid, single_upgrade_root};
use crate::{Plan, Result, managed, network::principal, supplier};
use canic_management_protocol::{children::Slot, stored as wire};

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
        || store.sources.iter().all(Option::is_none)
        || store.sources[children.len()..].iter().any(Option::is_some)
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
        let slot = crate::fleet_plan::SLOTS[index];
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
    ic_wire::native::encode(&expected(plan)?, wire::LIMITS).map_err(supplier)?;
    Ok(())
}

fn delivery<A: serde::Serialize>(
    store: &str,
    slot: Slot,
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
    if method != "observe_stored_children" {
        return Err(invalid());
    }
    let actual: wire::Custody =
        ic_wire::native::decode_response(wire::CUSTODY, bytes, wire::LIMITS).map_err(supplier)?;
    Ok(actual == expected(plan)?)
}
