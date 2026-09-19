//! Module: fleet_ensure::policy::independent_effects
//!
//! Responsibility: select independent prepared uploads or distinct pool imports.
//! Does not own: calls, hashing, journal mutation or retry.
//! Boundary: only a fresh bounded suffix sharing exact target authority may overlap.

use crate::fleet_ensure::model::{
    CurrentFleetProtocolAction, EffectRecord, EffectState, EnsureAction,
    MAX_INDEPENDENT_EFFECTS_IN_FLIGHT,
};
use canic_control_plane::ids::{TemplateId, TemplateVersion};

#[derive(Eq, PartialEq)]
struct ChunkAuthority<'a> {
    candid: &'a str,
    candid_sha256: &'a str,
    principal: &'a str,
    template_id: &'a TemplateId,
    version: &'a TemplateVersion,
}

fn authority(action: &EnsureAction) -> Option<ChunkAuthority<'_>> {
    let EnsureAction::FleetProtocol {
        action,
        candid,
        candid_sha256,
        principal,
        ..
    } = action
    else {
        return None;
    };
    let CurrentFleetProtocolAction::PublishStoreChunk { request } = action.as_ref() else {
        return None;
    };
    Some(ChunkAuthority {
        candid,
        candid_sha256,
        principal,
        template_id: &request.template_id,
        version: &request.version,
    })
}

/// Retained partial batches use the ordinary exact-observation recovery driver.
pub(in crate::fleet_ensure) fn batch_len(
    actions: &[&EnsureAction],
    effects: &[EffectRecord],
    start: usize,
) -> usize {
    let Some(first) = actions.get(start) else {
        return 0;
    };
    let pool = pool_authority(first);
    let chunk = authority(first);
    if pool.is_none() && chunk.is_none() {
        return 0;
    }
    if effects.len() != start
        || effects
            .iter()
            .any(|effect| effect.state != EffectState::Applied)
    {
        return 0;
    }
    if let Some(expected) = pool {
        let mut assets = std::collections::BTreeSet::new();
        return actions[start..]
            .iter()
            .take(MAX_INDEPENDENT_EFFECTS_IN_FLIGHT)
            .take_while(|action| {
                if pool_authority(action) != Some(expected) {
                    return false;
                }
                let EnsureAction::FleetProtocol { action, .. } = action else {
                    return false;
                };
                let CurrentFleetProtocolAction::ReconcilePoolAsset { request, .. } =
                    action.as_ref()
                else {
                    return false;
                };
                assets.insert(request.canister_id)
            })
            .count();
    }
    let Some(expected) = chunk else {
        return 0;
    };
    if start == 0 {
        return 0;
    }
    // Find the same payload's applied chunk zero, without crossing another action.
    let mut prepared = false;
    for action in actions[..start].iter().rev() {
        if authority(action).as_ref() != Some(&expected) {
            break;
        }
        let EnsureAction::FleetProtocol { action, .. } = action else {
            break;
        };
        if let CurrentFleetProtocolAction::PublishStoreChunk { request } = action.as_ref()
            && request.preparation.is_some()
            && request.chunk_index == 0
        {
            prepared = true;
            break;
        }
    }
    if !prepared {
        return 0;
    }
    let mut indices = std::collections::BTreeSet::new();
    actions[start..]
        .iter()
        .take(MAX_INDEPENDENT_EFFECTS_IN_FLIGHT)
        .take_while(|action| {
            if authority(action).as_ref() != Some(&expected) {
                return false;
            }
            let EnsureAction::FleetProtocol { action, .. } = action else {
                return false;
            };
            let CurrentFleetProtocolAction::PublishStoreChunk { request } = action.as_ref() else {
                return false;
            };
            request.preparation.is_none()
                && request.chunk_index > 0
                && indices.insert(request.chunk_index)
        })
        .count()
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct PoolAuthority<'a> {
    candid: &'a str,
    candid_sha256: &'a str,
    principal: &'a str,
}

fn pool_authority(action: &EnsureAction) -> Option<PoolAuthority<'_>> {
    let EnsureAction::FleetProtocol {
        action,
        candid,
        candid_sha256,
        principal,
        ..
    } = action
    else {
        return None;
    };
    matches!(
        action.as_ref(),
        CurrentFleetProtocolAction::ReconcilePoolAsset { .. }
    )
    .then_some(PoolAuthority {
        candid,
        candid_sha256,
        principal,
    })
}
