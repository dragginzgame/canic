//! One bounded anonymous observation of the Root selected by this fleet's reserve.
use super::{FleetOperation, network};
use crate::{Error, Result, SelectedFleet, supplier};
use canic::method::Contract;
use canic_management_protocol::stored::{ReadSpareCounts, SpareCounts};
use std::{collections::BTreeSet, time::Duration};

pub(super) async fn observe(
    selected: &SelectedFleet,
    operations: &[FleetOperation],
) -> Result<Option<SpareCounts>> {
    let Some(policy) = &selected.config.standby_pool else {
        return Ok(None);
    };
    let network = &selected.config.network;
    let binding = network.binding_id()?;
    let mut ids = BTreeSet::new();
    if let Some(id) = selected
        .config
        .canisters
        .get(&policy.root)
        .and_then(|slot| slot.id.as_deref())
    {
        ids.insert(id);
    }
    for op in operations {
        if op.network_id == binding
            && op.deployment.as_ref().is_some_and(|d| {
                d.app == selected.app && d.fleet == selected.fleet && d.canister == policy.root
            })
            && let Some(id) = op.canister.as_deref()
        {
            ids.insert(id);
        }
    }
    if ids.len() > 1 {
        return Err(Error::Invalid("conflicting reserve Root bindings".into()));
    }
    let Some(id) = ids.first() else {
        return Ok(None);
    };
    let request = ReadSpareCounts::encode_request(&()).map_err(supplier)?;
    let reply = tokio::time::timeout(
        Duration::from_secs(2),
        network::query(
            &network.client()?,
            network::principal(id)?,
            ReadSpareCounts::NAME,
            &request,
        ),
    )
    .await
    .map_err(|_| Error::Blocked("reserve observation timed out".into()))??;
    ReadSpareCounts::decode_reply(&reply)
        .map_err(supplier)?
        .map(Some)
        .ok_or_else(|| Error::Blocked("Root has no approved reserve policy".into()))
}
