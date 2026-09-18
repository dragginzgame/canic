//! Production Ledger mint and native withdrawal in one retained host operation.
//!
//! The destination is a minimal canister, not a managed Fleet lifecycle fixture.

use super::operator_mint_tests::funded_operator;
use crate::{
    fleet_ensure::{
        model::*,
        ops::{
            self, EffectObservation, EffectOutcome, EffectRetry, EnsurePaths, EnsurePlatform,
            NativeFundingObservation, native_funding_applied,
            operator_mint::{prepare_intent, transport::OperatorMintTransport},
        },
        view::OperatorFundingObservation,
        workflow::{self, EnsureWorkflowError, operator_mint},
    },
    test_support::temp_dir,
};
use candid::{CandidType, Nat, Principal};
use canic_core::cdk::utils::hash::{decode_hex, hex_bytes, sha256_hex};
use ic_testkit::{
    pic::CandidCallExt,
    pocket_ic::{CanisterSettings, CreateCanisterParams, PocketIc},
};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs, io,
    time::{SystemTime, UNIX_EPOCH},
};

const LEDGER: &str = "um5iw-rqaaa-aaaaq-qaaba-cai";

struct FundedCanister {
    pic: PocketIc,
    transport: OperatorMintTransport,
    destination: Principal,
    desired: DesiredFleet,
    fee: u128,
    calls: usize,
    duplicates: usize,
}

impl FundedCanister {
    fn balance(&self) -> u128 {
        self.transport
            .operator_balance(Principal::from_text(LEDGER).unwrap())
            .unwrap()
    }
}

impl EnsurePlatform for FundedCanister {
    type Error = io::Error;

    fn reinstall_authorities(
        &mut self,
        state: &FleetEnsureStateRecord,
    ) -> Result<Option<BTreeMap<String, RootManagementCanisterObservation>>, Self::Error> {
        let mut observed = self.observe("retained-native-funding", state)?;
        let live = observed.canisters.remove("treasury").unwrap().unwrap();
        Ok(Some(BTreeMap::from([(
            "treasury".into(),
            RootManagementCanisterObservation {
                live,
                name: "treasury".into(),
                subnet: self.pic.get_subnet(self.destination).unwrap().to_text(),
            },
        )])))
    }

    fn bind_reviewed_desired(&mut self, desired: &DesiredFleet) -> Result<(), Self::Error> {
        self.desired = desired.clone();
        Ok(())
    }

    fn observe_operator_funding(
        &mut self,
    ) -> Result<Option<OperatorFundingObservation>, Self::Error> {
        Ok(Some(OperatorFundingObservation {
            cycles_ledger: LEDGER.into(),
            ledger_fee_cycles: self.fee,
            operator_cycles: self.balance(),
        }))
    }

    fn observe(
        &mut self,
        _: &str,
        _: &FleetEnsureStateRecord,
    ) -> Result<FleetObservation, Self::Error> {
        let status = self
            .pic
            .canister_status(self.destination, Some(self.transport.operator().unwrap()))
            .unwrap();
        Ok(FleetObservation {
            additional_controlled_cycles: BTreeMap::new(),
            estate_funding_domains: BTreeMap::new(),
            ledger_fee_cycles: self.fee,
            operator_cycles: self.balance(),
            protocol_ready: BTreeMap::new(),
            canisters: BTreeMap::from([(
                "treasury".into(),
                Some(LiveCanister {
                    principal: self.destination.to_text(),
                    cycles: self.pic.cycle_balance(self.destination),
                    controllers: status
                        .settings
                        .controllers
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                    module_sha256: status.module_hash.map(hex_bytes),
                    canister_version: Some(status.version),
                    status: CanisterRuntimeStatus::Running,
                    reinstall_required: false,
                    root_owned_lifecycle: None,
                }),
            )]),
        })
    }

    fn observe_effect(
        &mut self,
        _: &str,
        action: &EnsureAction,
        record: &EffectRecord,
        _: &FleetEnsureStateRecord,
    ) -> Result<EffectObservation, Self::Error> {
        let EnsureAction::Fund {
            amount,
            expected_post_cycles,
            funding_deficit_cycles,
            funding_margin_cycles,
            ..
        } = action
        else {
            panic!("only native withdrawal is in this fixture")
        };
        let live_cycles = Some(self.pic.cycle_balance(self.destination));
        Ok(EffectObservation {
            provisioning_progress: None,
            provisioning_failure: None,
            estate_funding_required: None,
            applied: record.receipt.is_some()
                && native_funding_applied(NativeFundingObservation {
                    amount: *amount,
                    expected_post_cycles: *expected_post_cycles,
                    funding_deficit_cycles: *funding_deficit_cycles,
                    funding_margin_cycles: *funding_margin_cycles,
                    pre_cycles: record.pre_cycles,
                    live_cycles,
                }),
            post_cycles: live_cycles,
            progress_identity: format!("withdraw:{:?}", record.receipt),
            retry: EffectRetry::None,
        })
    }

    fn action_cycles(
        &mut self,
        _: &EnsureAction,
        _: &FleetEnsureStateRecord,
    ) -> Result<Option<u128>, Self::Error> {
        Ok(Some(self.pic.cycle_balance(self.destination)))
    }
    fn action_destination_cycles(
        &mut self,
        _: &EnsureAction,
        _: &FleetEnsureStateRecord,
    ) -> Result<Option<u128>, Self::Error> {
        Ok(None)
    }
    fn pace_effect_observation(&mut self, _: &EnsureAction, _: u32) {}

    fn apply(
        &mut self,
        _: &str,
        action: &EnsureAction,
        _: &EffectRecord,
        _: &FleetEnsureStateRecord,
    ) -> Result<EffectOutcome, Self::Error> {
        let EnsureAction::Fund {
            amount,
            created_at_time,
            ledger,
            principal,
            ..
        } = action
        else {
            panic!("only native withdrawal is in this fixture")
        };
        assert_eq!(principal, &self.destination.to_text());
        self.calls += 1;
        let reply: Result<Nat, WithdrawError> = self
            .pic
            .update_candid_as(
                Principal::from_text(ledger).unwrap(),
                self.transport.operator().unwrap(),
                "withdraw",
                (WithdrawArgs {
                    amount: Nat::from(*amount),
                    created_at_time: Some(*created_at_time),
                    from_subaccount: None,
                    to: self.destination,
                },),
            )
            .unwrap();
        let block = match reply {
            Ok(_) => {
                assert_eq!(self.calls, 1, "a retry must not pay again");
                return Err(io::Error::other(
                    "fixture lost the successful withdrawal reply",
                ));
            }
            Err(WithdrawError::Duplicate { duplicate_of }) => {
                self.duplicates += 1;
                duplicate_of
            }
            Err(error) => panic!("unexpected withdrawal outcome: {error:?}"),
        };
        Ok(EffectOutcome {
            created_principal: None,
            post_cycles: Some(self.pic.cycle_balance(self.destination)),
            receipt: Some(block.to_string()),
        })
    }
}

#[test]
#[ignore = "governed production Ledger mint, native funding and lost-reply proof"]
#[expect(
    clippy::too_many_lines,
    reason = "one retained operation binds mint credit, original withdrawal, conservation and replay"
)]
fn governed_pocketic_mint_credit_resumes_original_native_withdrawal() {
    let (pic, transport) = funded_operator();
    let operator = transport.operator().unwrap();
    let ledger = Principal::from_text(LEDGER).unwrap();
    let fee: Nat = pic.query_candid(ledger, "icrc1_fee", ()).unwrap();
    let fee: u128 = fee.0.try_into().unwrap();
    let destination = pic
        .create_canister_with_params(
            None,
            CreateCanisterParams {
                cycles: Some(1_000_000_000_000),
                settings: Some(CanisterSettings {
                    controllers: Some(vec![operator]),
                    ..CanisterSettings::default()
                }),
                ..CreateCanisterParams::default()
            },
        )
        .unwrap();
    let wasm = b"\0asm\x01\0\0\0";
    pic.install_canister(destination, wasm.to_vec(), vec![], Some(operator));
    let root = temp_dir("mint-native-recovery");
    fs::create_dir_all(&root).unwrap();
    let wasm_path = root.join("treasury.wasm");
    fs::write(&wasm_path, wasm).unwrap();
    let desired: DesiredFleet = serde_json::from_value(serde_json::json!({
        "canisters": [{"controllers": [operator.to_text()], "initial_cycles": "2T", "minimum_cycles": "2T",
            "kind": "coordinator", "name": "treasury", "presence": "present", "principal": destination.to_text(),
            "replace": false, "subnet": pic.get_subnet(destination).unwrap().to_text(), "wasm": wasm_path}],
        "cycles_ledger": LEDGER, "environment": "local", "fleet": "mint-fleet",
        "ledger_fee_cycles": fee.to_string(), "management_creation_fee_cycles": "0",
        "material_cycle_threshold": "1B", "maximum_observation_burn_cycles": "1B",
        "maximum_stalled_observations": 2, "maximum_update_burn_cycles": "2B",
        "operator": operator.to_text(), "schema_version": 1, "treasury": "treasury"
    })).unwrap();
    let mut platform = FundedCanister {
        pic,
        transport,
        destination,
        desired: desired.clone(),
        fee,
        calls: 0,
        duplicates: 0,
    };
    assert_eq!(platform.balance(), 0);
    let source = sha256_hex(b"minimal real native funding recovery");
    let mut plan = workflow::plan(&root, &desired, &source, "mint-fleet", now(), &mut platform)
        .unwrap()
        .plan;
    let paths = EnsurePaths::under(&root, "local", "mint-fleet");
    crate::fleet_ensure::tests::retain_recorded_retirement(
        &mut plan,
        vec![RootManagementBinding {
            controllers: vec![operator.to_text()],
            module_sha256: sha256_hex(wasm),
            name: "treasury".into(),
            principal: destination.to_text(),
            subnet: platform.pic.get_subnet(destination).unwrap().to_text(),
        }],
    );
    ops::write_plan(&paths, &plan).unwrap();
    let original_plan = fs::read(&paths.plan).unwrap();
    let actions = workflow::ordered_actions(&plan);
    assert_eq!(actions.len(), 1);
    let state = ops::read_state(&paths, "mint-fleet").unwrap();
    let intent = ops::effect_preparation::prepare_effect(
        &mut platform,
        &plan.operation_id,
        actions[0],
        &state,
    )
    .unwrap()
    .record;
    // The released bug could retain an underfunded original intent. Current
    // fresh admission rejects it; reproduce that retained evidence explicitly.
    let journal = FleetEnsureJournalRecord {
        funding_reviews: vec![],
        successor_phases: vec![],
        completion: FleetEnsureCompletion::InProgress,
        estate_funding_required: None,
        effects: vec![intent.clone()],
        fleet: "mint-fleet".into(),
        initial_controlled_cycles: platform.pic.cycle_balance(destination),
        initial_estate_funding_cycles_by_root: BTreeMap::new(),
        initial_operator_cycles: 0,
        operation_id: plan.operation_id.clone(),
        plan_sha256: plan.plan_sha256.clone(),
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        stalled_observations: 0,
    };
    ops::write_journal(&paths, &journal).unwrap();
    let review = workflow::plan(&root, &desired, &source, "mint-fleet", now(), &mut platform)
        .unwrap()
        .funding_review
        .unwrap();
    assert!(matches!(review.pause, FundingPauseRecord::Operator(_)));
    let authority = crate::fleet_ensure::model::operator_mint::OperatorMintAuthority {
        operation_id: digest(&plan.operation_id),
        plan_sha256: digest(&plan.plan_sha256),
        funding_review_sha256: digest(&review.review_sha256),
        network_identity_sha256: platform.transport.network_identity(),
        operator,
        icp_ledger: Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai").unwrap(),
        cmc: Principal::from_text("rkp4c-7iaaa-aaaaa-aaaca-cai").unwrap(),
        cycles_ledger: ledger,
    };
    let mint = prepare_intent(authority.clone(), 100_000_000, 10_000, now()).unwrap();
    let reviewed = operator_mint::review(&paths, &mint).unwrap();
    let credited = operator_mint::execution::apply_blocking(
        &paths,
        &authority,
        &reviewed.review_sha256,
        &platform.transport,
    )
    .unwrap();
    let credit = credited.receipt.as_ref().unwrap().net_credit_cycles;
    assert_eq!(platform.balance(), credit);
    assert_eq!(fs::read(&paths.plan).unwrap(), original_plan);
    let after = ops::read_journal(&paths).unwrap().unwrap();
    assert_eq!(after.effects, journal.effects);
    assert_eq!(after.initial_operator_cycles, 0);
    assert!(matches!(
        workflow::apply(
            &root,
            &desired,
            &source,
            "mint-fleet",
            &plan.plan_sha256,
            &mut platform
        ),
        Err(EnsureWorkflowError::Platform(_))
    ));
    let after_loss = platform.balance();
    let complete = workflow::apply(
        &root,
        &desired,
        &source,
        "mint-fleet",
        &plan.plan_sha256,
        &mut platform,
    )
    .unwrap();
    assert!(complete.terminal);
    assert_eq!(platform.calls, 2);
    assert_eq!(platform.duplicates, 1);
    assert_eq!(platform.balance(), after_loss);
    assert_eq!(
        credit - after_loss,
        plan.conservation.maximum_operator_debit_cycles
    );
    assert_eq!(
        complete.actual_conservation.unwrap().operator_debit_cycles,
        credit - after_loss
    );
    assert!(platform.pic.cycle_balance(destination) >= 2_000_000_000_000);
    let retained = ops::read_journal(&paths).unwrap().unwrap();
    assert_eq!(retained.effects[0].action_sha256, intent.action_sha256);
    assert_eq!(retained.initial_operator_cycles, 0);
    let journal_bytes = fs::read(&paths.journal).unwrap();
    assert_eq!(
        operator_mint::execution::apply_blocking(
            &paths,
            &authority,
            &reviewed.review_sha256,
            &platform.transport
        )
        .unwrap(),
        credited
    );
    assert!(
        workflow::apply(
            &root,
            &desired,
            &source,
            "mint-fleet",
            &plan.plan_sha256,
            &mut platform
        )
        .unwrap()
        .terminal
    );
    assert_eq!(platform.calls, 2);
    assert_eq!(platform.balance(), after_loss);
    assert_eq!(fs::read(&paths.journal).unwrap(), journal_bytes);
    fs::remove_dir_all(root).unwrap();
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .try_into()
        .unwrap()
}
fn digest(text: &str) -> [u8; 32] {
    decode_hex(text).unwrap().try_into().unwrap()
}

#[derive(CandidType)]
struct WithdrawArgs {
    amount: Nat,
    created_at_time: Option<u64>,
    from_subaccount: Option<[u8; 32]>,
    to: Principal,
}

#[derive(CandidType, Debug, Deserialize)]
enum WithdrawError {
    BadFee {
        expected_fee: Nat,
    },
    CreatedInFuture {
        ledger_time: u64,
    },
    Duplicate {
        duplicate_of: Nat,
    },
    FailedToWithdraw {
        fee_block: Option<Nat>,
        rejection_code: RejectionCode,
        rejection_reason: String,
    },
    GenericError {
        error_code: Nat,
        message: String,
    },
    InsufficientFunds {
        balance: Nat,
    },
    InvalidReceiver {
        receiver: Principal,
    },
    TemporarilyUnavailable,
    TooOld,
}

#[derive(CandidType, Debug, Deserialize)]
enum RejectionCode {
    CanisterError,
    CanisterReject,
    DestinationInvalid,
    NoError,
    SysFatal,
    SysTransient,
    Unknown,
}
