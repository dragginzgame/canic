//! Seal admission checks exact current wire contracts before any transport call.

use super::*;
use crate::fleet_ensure::model::DesiredCanisterKind;
use candid::types::{FuncMode, Function, Type, TypeInner, internal::TypeContainer};

fn contract_text(kind: DesiredCanisterKind) -> String {
    let mut types = TypeContainer::new();
    let command = types.add::<Command>();
    let command_result = types.add::<Result<CommandResponse, canic_core::dto::error::Error>>();
    let status = types.add::<Status>();
    let status_result = types.add::<Result<StatusResponse, canic_core::dto::error::Error>>();
    let method = |request, response, modes| -> Type {
        TypeInner::Func(Function {
            args: vec![request],
            rets: vec![response],
            modes,
        })
        .into()
    };
    let (command_name, status_name) = contract::methods(kind).unwrap();
    let actor = TypeInner::Service(vec![
        (
            command_name.into(),
            method(command, command_result, Vec::new()),
        ),
        (
            status_name.into(),
            method(status, status_result, vec![FuncMode::Query]),
        ),
    ])
    .into();
    candid::pretty::candid::compile(&types.env, &Some(actor))
}

#[test]
fn seal_contract_requires_exact_selectors_modes_and_payloads() {
    for kind in [DesiredCanisterKind::Coordinator, DesiredCanisterKind::Root] {
        let text = contract_text(kind);
        assert!(contract::verify(&text, kind).is_some());
        for changed in [
            text.replace(" query", ""),
            text.replace(" query", " composite_query"),
            text.replace("PrepareAuthoritySnapshot", "DifferentSnapshot"),
            text.replace("AuthorityRestore :", "DifferentStatus :"),
            text.replace(
                "authority_canister : principal",
                "authority_canister : text",
            ),
            text.replace("operation_id : blob", "operation_id : text"),
            text.replace(
                "history_total_num_changes : opt nat64",
                "history_total_num_changes : opt nat",
            ),
            text.replace("Err :", "DifferentError :"),
        ] {
            assert_ne!(changed, text);
            assert!(contract::verify(&changed, kind).is_none(), "{changed}");
        }
        for kind in [DesiredCanisterKind::Store, DesiredCanisterKind::Pool] {
            assert!(contract::verify(&text, kind).is_none());
        }
    }
}

#[test]
fn seal_authority_rejects_a_bound_but_incompatible_contract_before_transport() {
    let directory = crate::test_support::temp_dir("authority-seal-contract");
    std::fs::create_dir_all(&directory).unwrap();
    let kind = DesiredCanisterKind::Root;
    let good = contract_text(kind);
    for (text, valid) in [(good.clone(), true), (good.replace(" query", ""), false)] {
        std::fs::write(directory.join("root.did"), &text).unwrap();
        let action = EnsureAction::SealAuthority {
            authority_kind: kind,
            candid: "root.did".into(),
            candid_sha256: canic_core::cdk::utils::hash::sha256_hex(text.as_bytes()),
            name: "root".into(),
            principal: Principal::from_slice(&[42]).to_text(),
        };
        let result = authority(&directory, &"11".repeat(32), &action);
        if valid {
            assert!(result.is_ok());
        } else {
            assert!(matches!(
                result,
                Err(CurrentProtocolError::AuthoritySealContract { .. })
            ));
        }
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn shipped_coordinator_exposes_the_maintained_seal_contract() {
    let text = include_str!("../../../../../canic/candid/fleet_coordinator.did");
    assert!(contract::verify(text, DesiredCanisterKind::Coordinator).is_some());
}

#[test]
fn fence_observation_preserves_complete_history_and_resume_receipts() {
    let principal = Principal::from_slice(&[42]);
    for phase in [
        AuthorityRestoreFencePhase::Open,
        AuthorityRestoreFencePhase::Sealed,
    ] {
        for fields in 0..8 {
            let status = AuthorityRestoreFenceStatusResponse {
                authority_canister: principal,
                phase,
                operation_id: (fields & 1 != 0).then_some([11; 32]),
                history_total_num_changes: (fields & 2 != 0).then_some(7),
                changed_at_ns: (fields & 4 != 0).then_some(99),
            };
            let accepted =
                fields == 7 || (phase == AuthorityRestoreFencePhase::Open && fields == 0);
            if accepted {
                validate_status(principal, &status).unwrap();
            } else {
                assert!(matches!(
                    validate_status(principal, &status),
                    Err(CurrentProtocolError::ResponseMismatch)
                ));
            }
            assert!(matches!(
                validate_status(Principal::from_slice(&[99]), &status),
                Err(CurrentProtocolError::ResponseMismatch)
            ));
        }
    }
}

#[test]
fn seal_reconciliation_requires_this_operation_and_complete_receipt() {
    let authority = Authority {
        candid: "unused.did".into(),
        command: canic_core::protocol::CANIC_ROOT_COMMAND,
        status: canic_core::protocol::CANIC_ROOT_STATUS,
        principal: Principal::from_slice(&[42]),
        operation: [11; 32],
        name: "root",
    };
    let sealed = AuthorityRestoreFenceStatusResponse {
        authority_canister: authority.principal,
        phase: AuthorityRestoreFencePhase::Sealed,
        operation_id: Some(authority.operation),
        history_total_num_changes: Some(7),
        changed_at_ns: Some(99),
    };
    // A lost seal reply is reconciled by its exact complete read-only receipt.
    assert!(observation(&authority, &sealed).unwrap().applied);
    let resumed = AuthorityRestoreFenceStatusResponse {
        phase: AuthorityRestoreFencePhase::Open,
        ..sealed
    };
    let retry = observation(&authority, &resumed).unwrap();
    assert!(!retry.applied);
    assert_eq!(retry.retry, EffectRetry::ReplayExactIssuedCommand);
    for invalid in [
        AuthorityRestoreFenceStatusResponse {
            operation_id: Some([12; 32]),
            ..sealed
        },
        AuthorityRestoreFenceStatusResponse {
            history_total_num_changes: None,
            ..sealed
        },
        AuthorityRestoreFenceStatusResponse {
            authority_canister: Principal::from_slice(&[99]),
            ..sealed
        },
    ] {
        assert!(matches!(
            observation(&authority, &invalid),
            Err(CurrentProtocolError::ResponseMismatch)
        ));
    }
}
