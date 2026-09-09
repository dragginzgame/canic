//! Module: fleet_ensure::ops::install_history
//!
//! Responsibility: verify that an observed version advance was the reviewed reinstall.
//! Does not own: install intent, retries or lifecycle orchestration.
//! Boundary: replicated Root observations must agree with authenticated management status.

use crate::{
    canister_protocol::call_with_candid,
    fleet_ensure::{
        model::{LiveCanister, ReinstallHistoryWitness},
        ops::current_protocol::CurrentProtocolError,
    },
    icp::IcpCli,
};
use candid::{CandidType, Principal};
use canic_core::dto::canister::{CanisterHistoryResponse, CanisterInspectionRequest};
use serde::Deserialize;
use std::path::Path;

#[derive(CandidType)]
enum Command {
    InspectCanisterHistory(CanisterInspectionRequest),
}
#[derive(CandidType, Deserialize)]
enum Response {
    InspectCanisterHistory(CanisterHistoryResponse),
}

/// Recovery decision backed by replicated history, including non-deployment version changes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum HistoryEffect {
    Applied,
    NotApplied,
    Conflict,
}

#[derive(CandidType, Deserialize)]
struct History {
    module_hash: Option<Vec<u8>>,
    controllers: Vec<Principal>,
    recent_changes: Vec<Change>,
}

#[derive(CandidType, Deserialize)]
struct Change {
    canister_version: u64,
    origin: Option<Origin>,
    details: Option<Details>,
}

#[derive(CandidType, Deserialize)]
enum Origin {
    #[serde(rename = "from_user")]
    FromUser { user_id: Principal },
}

#[derive(CandidType, Deserialize)]
enum Details {
    #[serde(rename = "code_deployment")]
    CodeDeployment { mode: Mode, module_hash: Vec<u8> },
}

#[derive(CandidType, Deserialize)]
enum Mode {
    #[serde(rename = "install")]
    Install,
    #[serde(rename = "reinstall")]
    Reinstall,
    #[serde(rename = "upgrade")]
    Upgrade,
}

pub(super) fn observe(
    icp: &IcpCli,
    root: &Path,
    witness: &ReinstallHistoryWitness,
    operator: Principal,
    before: u64,
    live: &LiveCanister,
) -> Result<HistoryEffect, CurrentProtocolError> {
    let path = root.join(&witness.candid);
    let bytes =
        std::fs::read(&path).map_err(|e| CurrentProtocolError::Configuration(e.to_string()))?;
    if canic_core::cdk::utils::hash::sha256_hex(&bytes) != witness.candid_sha256 {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    let principal = Principal::from_text(&live.principal)
        .map_err(|_| CurrentProtocolError::ResponseMismatch)?;
    let witness_principal = Principal::from_text(&witness.authority.principal)
        .map_err(|_| CurrentProtocolError::ResponseMismatch)?;
    let Response::InspectCanisterHistory(response) = call_with_candid(
        icp,
        &path,
        witness_principal,
        canic_core::protocol::CANIC_ROOT_COMMAND,
        &Command::InspectCanisterHistory(CanisterInspectionRequest {
            canister_id: principal,
        }),
    )?;
    if response.canister_id != principal {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    let history: History = candid::decode_one(&response.history_candid)
        .map_err(|_| CurrentProtocolError::ResponseMismatch)?;
    Ok(classify_history(&history, operator, before, live))
}

/// Correlate deployment history with both module identities in the reviewed transition.
pub(super) fn reconcile(
    history: HistoryEffect,
    requested_hash: &str,
    prior_hash: &str,
    before: u64,
    live: &LiveCanister,
) -> HistoryEffect {
    match history {
        HistoryEffect::Applied
            if live.module_sha256.as_deref() == Some(requested_hash)
                && live
                    .canister_version
                    .is_some_and(|version| version > before) =>
        {
            HistoryEffect::Applied
        }
        HistoryEffect::NotApplied if live.module_sha256.as_deref() == Some(prior_hash) => {
            HistoryEffect::NotApplied
        }
        _ => HistoryEffect::Conflict,
    }
}

fn classify_history(
    history: &History,
    operator: Principal,
    before: u64,
    live: &LiveCanister,
) -> HistoryEffect {
    let Some(expected_hash) = live.module_sha256.as_deref() else {
        return HistoryEffect::Conflict;
    };
    let mut controllers = history
        .controllers
        .iter()
        .map(Principal::to_text)
        .collect::<Vec<_>>();
    controllers.sort();
    let mut expected_controllers = live.controllers.clone();
    expected_controllers.sort();
    let hash_matches = history
        .module_hash
        .as_ref()
        .is_some_and(|hash| canic_core::cdk::utils::hash::hex_bytes(hash) == expected_hash);
    if !hash_matches
        || controllers != expected_controllers
        || live.canister_version.is_none_or(|v| v < before)
    {
        return HistoryEffect::Conflict;
    }
    let Some(change) = history.recent_changes.last() else {
        return HistoryEffect::Conflict;
    };
    if change.canister_version <= before {
        return HistoryEffect::NotApplied;
    }
    if matches_history(history, operator, expected_hash, before, live) {
        HistoryEffect::Applied
    } else {
        HistoryEffect::Conflict
    }
}

fn matches_history(
    history: &History,
    operator: Principal,
    expected_hash: &str,
    before: u64,
    live: &LiveCanister,
) -> bool {
    let Some(change) = history.recent_changes.last() else {
        return false;
    };
    let Some(Details::CodeDeployment {
        mode: Mode::Reinstall,
        module_hash,
    }) = &change.details
    else {
        return false;
    };
    let actor_matches =
        matches!(&change.origin, Some(Origin::FromUser { user_id }) if *user_id == operator);
    let version_matches = live.canister_version.is_some_and(|current| {
        change.canister_version > before && change.canister_version <= current
    });
    let hash_matches = canic_core::cdk::utils::hash::hex_bytes(module_hash) == expected_hash
        && history.module_hash.as_ref() == Some(module_hash);
    let mut controllers = history
        .controllers
        .iter()
        .map(Principal::to_text)
        .collect::<Vec<_>>();
    controllers.sort();
    let mut expected_controllers = live.controllers.clone();
    expected_controllers.sort();
    actor_matches && version_matches && hash_matches && controllers == expected_controllers
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fleet_ensure::model::CanisterRuntimeStatus;

    #[test]
    fn management_history_wire_labels_decode_into_the_exact_reinstall_proof() {
        let hash_blob = r"\11".repeat(32);
        let wire = format!(
            r#"(record {{
            module_hash = opt blob "{hash_blob}";
            controllers = vec {{ principal "2vxsx-fae" }};
            recent_changes = vec {{ record {{
                canister_version = 11 : nat64;
                origin = variant {{ from_user = record {{ user_id = principal "2vxsx-fae" }} }};
                details = opt variant {{ code_deployment = record {{
                    mode = variant {{ reinstall }};
                    module_hash = blob "{hash_blob}"
                }} }}
            }} }}
        }})"#
        );
        let (environment, actor) = candid_parser::utils::CandidSource::Text(
            r"
            type Origin = variant {
                from_user : record { user_id : principal };
                from_canister : record { canister_id : principal };
            };
            type Details = variant {
                creation : reserved; code_uninstall;
                controllers_change : reserved; load_snapshot : reserved;
                rename_canister : reserved;
                code_deployment : record {
                    mode : variant { install; reinstall; upgrade };
                    module_hash : blob;
                };
            };
            service : { history : () -> (record {
                module_hash : opt blob; controllers : vec principal;
                recent_changes : vec record {
                    canister_version : nat64; origin : Origin; details : opt Details;
                };
            }) query }
        ",
        )
        .load()
        .unwrap();
        let types = &environment
            .get_method(actor.as_ref().unwrap(), "history")
            .unwrap()
            .rets;
        let encode = |wire: &str| {
            candid_parser::parse_idl_args(wire)
                .unwrap()
                .to_bytes_with_types(&environment, types)
                .unwrap()
        };
        let bytes = encode(&wire);
        let history: History = candid::decode_one(&bytes).unwrap();
        assert!(matches!(
            history.recent_changes[0].origin,
            Some(Origin::FromUser { .. })
        ));
        assert!(matches!(
            history.recent_changes[0].details,
            Some(Details::CodeDeployment {
                mode: Mode::Reinstall,
                ..
            })
        ));
        assert_eq!(history.module_hash, Some(vec![0x11; 32]));
        let foreign = wire
            .replace("from_user", "from_canister")
            .replace("user_id", "canister_id");
        let foreign: History = candid::decode_one(&encode(&foreign)).unwrap();
        assert!(foreign.recent_changes[0].origin.is_none());
    }

    #[test]
    fn changed_wasm_history_binds_both_reviewed_modules() {
        let mut live = LiveCanister {
            canister_version: Some(10),
            controllers: vec![Principal::anonymous().to_text()],
            cycles: 1,
            module_sha256: Some("11".repeat(32)),
            principal: Principal::management_canister().to_text(),
            reinstall_required: false,
            root_owned_lifecycle: None,
            status: CanisterRuntimeStatus::Stopped,
        };
        let prior = "11".repeat(32);
        let requested = "22".repeat(32);
        assert_eq!(
            reconcile(HistoryEffect::NotApplied, &requested, &prior, 10, &live),
            HistoryEffect::NotApplied
        );
        live.canister_version = Some(20);
        // Read-only replicated observations may advance the version before deployment.
        assert_eq!(
            reconcile(HistoryEffect::NotApplied, &requested, &prior, 10, &live),
            HistoryEffect::NotApplied
        );
        live.module_sha256 = Some(requested.clone());
        assert_eq!(
            reconcile(HistoryEffect::NotApplied, &requested, &prior, 10, &live),
            HistoryEffect::Conflict
        );
        assert_eq!(
            reconcile(HistoryEffect::Applied, &requested, &prior, 10, &live),
            HistoryEffect::Applied
        );
        assert_eq!(
            reconcile(HistoryEffect::Conflict, &requested, &prior, 10, &live),
            HistoryEffect::Conflict
        );
        live.canister_version = Some(10);
        assert_eq!(
            reconcile(HistoryEffect::Applied, &requested, &prior, 10, &live),
            HistoryEffect::Conflict
        );
        live.module_sha256 = Some("33".repeat(32));
        assert_eq!(
            reconcile(HistoryEffect::NotApplied, &requested, &prior, 10, &live),
            HistoryEffect::Conflict
        );
    }

    #[test]
    fn same_wasm_recovery_requires_deployment_history_not_a_controller_change() {
        let operator = Principal::anonymous();
        let mut live = LiveCanister {
            canister_version: Some(11),
            controllers: vec![operator.to_text()],
            cycles: 1,
            module_sha256: Some("11".repeat(32)),
            principal: Principal::management_canister().to_text(),
            reinstall_required: false,
            root_owned_lifecycle: None,
            status: CanisterRuntimeStatus::Running,
        };
        let mut history = History {
            module_hash: Some(vec![0x11; 32]),
            controllers: vec![operator],
            recent_changes: vec![Change {
                canister_version: 11,
                origin: Some(Origin::FromUser { user_id: operator }),
                details: None,
            }],
        };
        assert!(!matches_history(
            &history,
            operator,
            &"11".repeat(32),
            10,
            &live
        ));
        history.recent_changes[0].details = Some(Details::CodeDeployment {
            mode: Mode::Reinstall,
            module_hash: vec![0x11; 32],
        });
        assert!(matches_history(
            &history,
            operator,
            &"11".repeat(32),
            10,
            &live
        ));
        assert!(!matches_history(
            &history,
            operator,
            &"11".repeat(32),
            11,
            &live
        ));
        assert!(!matches_history(
            &history,
            Principal::management_canister(),
            &"11".repeat(32),
            10,
            &live
        ));
        assert!(!matches_history(
            &history,
            operator,
            &"22".repeat(32),
            10,
            &live
        ));
        assert_eq!(
            classify_history(&history, operator, 10, &live),
            HistoryEffect::Applied
        );
        live.canister_version = Some(99);
        assert_eq!(
            classify_history(&history, operator, 11, &live),
            HistoryEffect::NotApplied
        );
        history.controllers.clear();
        assert_eq!(
            classify_history(&history, operator, 11, &live),
            HistoryEffect::Conflict
        );
        history.controllers.push(operator);
        assert_eq!(
            classify_history(&history, Principal::management_canister(), 10, &live),
            HistoryEffect::Conflict
        );
        history.recent_changes[0].details = Some(Details::CodeDeployment {
            mode: Mode::Upgrade,
            module_hash: vec![0x11; 32],
        });
        assert!(!matches_history(
            &history,
            operator,
            &"11".repeat(32),
            10,
            &live
        ));
    }
}
