//! Decode replicated management history into exact Root effect evidence.

use crate::view::canister_pool::{PoolImportHistoryKind, PoolImportHistoryView};
use canic_core::control_plane_support::error::InternalError;
use ic_platform::{
    management_canister_types::{
        CanisterInfoResult, ChangeDetails, ChangeOrigin, FromCanisterRecord,
    },
    principal::ExternalPrincipal as Principal,
};

pub(super) fn decode(
    canister_id: Principal,
    bytes: &[u8],
) -> Result<PoolImportHistoryView, InternalError> {
    let mut history: CanisterInfoResult =
        ic_platform::candid::decoder::decode_one(bytes).map_err(|_| InternalError::conflict())?;
    if history.recent_changes.len() != 1 {
        return Err(InternalError::conflict());
    }
    let change = history
        .recent_changes
        .pop()
        .ok_or_else(InternalError::conflict)?;
    let ChangeOrigin::FromCanister(FromCanisterRecord {
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
    use ic_platform::management_canister_types::{
        Change, CodeDeploymentMode, CodeDeploymentRecord, ControllersChangeRecord, FromUserRecord,
    };

    fn history(root: Principal, details: ChangeDetails) -> CanisterInfoResult {
        CanisterInfoResult {
            controllers: vec![root],
            module_hash: None,
            total_num_changes: 4,
            recent_changes: vec![Change {
                canister_version: 13,
                timestamp_nanos: 1,
                origin: ChangeOrigin::FromCanister(FromCanisterRecord {
                    canister_id: root,
                    canister_version: Some(44),
                }),
                details: Some(details),
            }],
        }
    }

    #[test]
    fn capacity_import_history_admits_exact_controllers_and_refuses_deployment() {
        let root = Principal::from_slice(&[1; 10]);
        let other = Principal::from_slice(&[2; 10]);
        let mut history = history(
            root,
            ChangeDetails::ControllersChange(ControllersChangeRecord {
                controllers: vec![root],
            }),
        );
        let bytes = ic_platform::candid::contract::encode_one(&history).unwrap();
        assert_eq!(
            decode(other, &bytes).unwrap().kind,
            PoolImportHistoryKind::Controllers
        );
        history.controllers = vec![other];
        let bytes = ic_platform::candid::contract::encode_one(&history).unwrap();
        assert!(decode(other, &bytes).is_err());
        history.controllers = vec![root];
        history.recent_changes[0].details =
            Some(ChangeDetails::CodeDeployment(CodeDeploymentRecord {
                mode: CodeDeploymentMode::Reinstall,
                module_hash: vec![0; 32],
            }));
        let bytes = ic_platform::candid::contract::encode_one(&history).unwrap();
        assert!(decode(other, &bytes).is_err());
    }

    #[test]
    fn capacity_import_history_binds_canister_origin_and_sender_version() {
        let root = Principal::from_slice(&[1; 10]);
        let source = Principal::from_slice(&[2; 10]);
        let mut history = history(root, ChangeDetails::CodeUninstall);
        let decoded = decode(
            source,
            &ic_platform::candid::contract::encode_one(&history).unwrap(),
        )
        .unwrap();
        assert_eq!(decoded.originator, root);
        assert_eq!(decoded.sender_canister_version, 44);
        assert_eq!(decoded.kind, PoolImportHistoryKind::Uninstall);
        history.recent_changes[0].origin = ChangeOrigin::FromUser(FromUserRecord { user_id: root });
        assert!(
            decode(
                source,
                &ic_platform::candid::contract::encode_one(&history).unwrap()
            )
            .is_err()
        );
        history.recent_changes[0].origin = ChangeOrigin::FromCanister(FromCanisterRecord {
            canister_id: root,
            canister_version: None,
        });
        assert!(
            decode(
                source,
                &ic_platform::candid::contract::encode_one(&history).unwrap()
            )
            .is_err()
        );
    }

    #[test]
    fn capacity_import_history_rejects_missing_evidence_and_invalid_hashes() {
        let root = Principal::from_slice(&[1; 10]);
        for case in 0..4 {
            let mut history = history(root, ChangeDetails::CodeUninstall);
            match case {
                0 => history.recent_changes.clear(),
                1 => history
                    .recent_changes
                    .push(history.recent_changes[0].clone()),
                2 => history.recent_changes[0].details = None,
                _ => history.module_hash = Some(vec![0; 31]),
            }
            assert!(
                decode(
                    root,
                    &ic_platform::candid::contract::encode_one(&history).unwrap()
                )
                .is_err()
            );
        }
        assert!(
            decode(
                root,
                &ic_platform::candid::contract::encode_one(()).unwrap()
            )
            .is_err()
        );
    }
}
