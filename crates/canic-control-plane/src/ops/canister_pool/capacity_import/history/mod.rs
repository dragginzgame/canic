//! Decode replicated management history into exact Root effect evidence.

use crate::view::canister_pool::{PoolImportHistoryKind, PoolImportHistoryView};
use canic_core::control_plane_support::error::InternalError;
use ic_platform::{
    candid::contract::CandidType, management_canister_types::ChangeDetails,
    principal::ExternalPrincipal as Principal,
};
use serde::Deserialize;

#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
struct History {
    module_hash: Option<Vec<u8>>,
    controllers: Vec<Principal>,
    recent_changes: Vec<Change>,
}

#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
struct Change {
    canister_version: u64,
    origin: Option<Origin>,
    details: Option<ChangeDetails>,
}

#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
enum Origin {
    #[serde(rename = "from_canister")]
    FromCanister {
        canister_id: Principal,
        canister_version: Option<u64>,
    },
}

pub(super) fn decode(
    canister_id: Principal,
    bytes: &[u8],
) -> Result<PoolImportHistoryView, InternalError> {
    let mut history: History =
        ic_platform::candid::decoder::decode_one(bytes).map_err(|_| InternalError::conflict())?;
    if history.recent_changes.len() != 1 {
        return Err(InternalError::conflict());
    }
    let change = history
        .recent_changes
        .pop()
        .ok_or_else(InternalError::conflict)?;
    let Some(Origin::FromCanister {
        canister_id: originator,
        canister_version: Some(sender_canister_version),
    }) = change.origin
    else {
        return Err(InternalError::conflict());
    };
    history.controllers.sort_unstable();
    let kind = match change.details {
        Some(ChangeDetails::ControllersChange(record)) => {
            let mut controllers = record.controllers;
            controllers.sort_unstable();
            if controllers != history.controllers {
                return Err(InternalError::conflict());
            }
            PoolImportHistoryKind::Controllers
        }
        Some(ChangeDetails::CodeUninstall) => PoolImportHistoryKind::Uninstall,
        _ => return Err(InternalError::conflict()),
    };
    Ok(PoolImportHistoryView {
        canister_id,
        canister_version: change.canister_version,
        module_sha256: history
            .module_hash
            .map(|hash| hash.try_into().map_err(|_| InternalError::conflict()))
            .transpose()?,
        controllers: history.controllers,
        originator,
        sender_canister_version,
        kind,
    })
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_import_history_admits_exact_controllers_and_refuses_deployment() {
        let root = Principal::from_slice(&[1; 10]);
        let other = Principal::from_slice(&[2; 10]);
        let mut history = History {
            controllers: vec![root],
            module_hash: None,
            recent_changes: vec![Change {
                canister_version: 13,
                origin: Some(Origin::FromCanister {
                    canister_id: root,
                    canister_version: Some(44),
                }),
                details: Some(
                    serde_json::from_value(serde_json::json!({
                        "controllers_change": { "controllers": [root.to_text()] }
                    }))
                    .unwrap(),
                ),
            }],
        };
        let bytes = ic_platform::candid::contract::encode_one(&history).unwrap();
        assert_eq!(
            decode(other, &bytes).unwrap().kind,
            PoolImportHistoryKind::Controllers
        );
        history.controllers = vec![other];
        let bytes = ic_platform::candid::contract::encode_one(&history).unwrap();
        assert!(decode(other, &bytes).is_err());

        history.controllers = vec![root];
        history.recent_changes[0].details = Some(
            serde_json::from_value(serde_json::json!({
                "code_deployment": { "mode": "reinstall", "module_hash": vec![0_u8; 32] }
            }))
            .unwrap(),
        );
        let bytes = ic_platform::candid::contract::encode_one(&history).unwrap();
        assert!(decode(other, &bytes).is_err());
    }

    #[derive(CandidType)]
    #[candid_path("ic_platform::candid::contract")]
    struct WireHistory {
        controllers: Vec<Principal>,
        module_hash: Option<Vec<u8>>,
        recent_changes: Vec<WireChange>,
        total_num_changes: u64,
    }

    #[derive(CandidType)]
    #[candid_path("ic_platform::candid::contract")]
    struct WireChange {
        canister_version: u64,
        origin: WireOrigin,
        details: Option<ChangeDetails>,
        timestamp_nanos: u64,
    }

    #[derive(CandidType, Deserialize)]
    #[candid_path("ic_platform::candid::contract")]
    enum WireOrigin {
        #[serde(rename = "from_user")]
        User { user_id: Principal },
        #[serde(rename = "from_canister")]
        Canister {
            canister_id: Principal,
            canister_version: Option<u64>,
        },
    }

    #[test]
    fn capacity_import_history_decodes_mandatory_wire_origin_without_accepting_user_wipes() {
        let root = Principal::from_slice(&[1; 10]);
        let source = Principal::from_slice(&[2; 10]);
        let mut history = WireHistory {
            controllers: vec![root],
            module_hash: None,
            total_num_changes: 4,
            recent_changes: vec![WireChange {
                canister_version: 13,
                timestamp_nanos: 1,
                origin: WireOrigin::Canister {
                    canister_id: root,
                    canister_version: Some(44),
                },
                details: Some(ChangeDetails::CodeUninstall),
            }],
        };
        let decoded = decode(
            source,
            &ic_platform::candid::contract::encode_one(&history).unwrap(),
        )
        .unwrap();
        assert_eq!(decoded.sender_canister_version, 44);
        history.recent_changes[0].origin = WireOrigin::User { user_id: root };
        assert!(
            decode(
                source,
                &ic_platform::candid::contract::encode_one(&history).unwrap()
            )
            .is_err()
        );
        history.recent_changes[0].origin = WireOrigin::Canister {
            canister_id: root,
            canister_version: None,
        };
        assert!(
            decode(
                source,
                &ic_platform::candid::contract::encode_one(&history).unwrap()
            )
            .is_err()
        );
    }

    #[test]
    fn capacity_import_history_keeps_sender_version_and_rejects_omitted_origin() {
        let root = Principal::from_slice(&[1; 10]);
        let canister_id = Principal::from_slice(&[2; 10]);
        let mut history = History {
            module_hash: None,
            controllers: vec![root],
            recent_changes: vec![Change {
                canister_version: 13,
                origin: Some(Origin::FromCanister {
                    canister_id: root,
                    canister_version: Some(44),
                }),
                details: Some(ChangeDetails::CodeUninstall),
            }],
        };
        let decoded = decode(
            canister_id,
            &ic_platform::candid::contract::encode_one(&history).unwrap(),
        )
        .unwrap();
        assert_eq!(decoded.originator, root);
        assert_eq!(decoded.sender_canister_version, 44);
        assert_eq!(decoded.kind, PoolImportHistoryKind::Uninstall);
        history.recent_changes[0].origin = None;
        assert!(
            decode(
                canister_id,
                &ic_platform::candid::contract::encode_one(&history).unwrap()
            )
            .is_err()
        );
    }
}
