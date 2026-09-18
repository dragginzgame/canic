//! Native signed evidence exercises admission without simulating IC calls or credit.

use super::test_support::{cbor, der_key, leb128, now, sign, signing_key};
use super::{
    CertificateVerificationError, CyclesLedgerBlock, DepositVerificationError as Error,
    ReceiptVerificationLimits, VerifiedCyclesDeposit, network_identity_sha256,
    verify_cycles_deposit,
};
use crate::fleet_ensure::{
    model::operator_mint::{
        OperatorMintAuthority, OperatorMintIntentRecord, OperatorMintNotificationOutcomeRecord,
    },
    ops::operator_mint::prepare_intent,
};
use candid::Principal;
use ic_agent::Agent;
use ic_certification::{Certificate, Delegation, HashTree, fork, labeled, leaf};
use ic_verify_bls_signature::PrivateKey;
use icrc_ledger_types::{icrc::generic_value::Value, icrc3::blocks::ICRC3DataCertificate};

struct Fixture {
    key: PrivateKey,
    agent: Agent,
    intent: OperatorMintIntentRecord,
    outcome: OperatorMintNotificationOutcomeRecord,
    chain: Vec<CyclesLedgerBlock>,
}

impl Fixture {
    fn new() -> Self {
        let key = signing_key();
        let root_key = der_key(&key);
        let agent = Agent::builder()
            .with_url("http://127.0.0.1:1")
            .build()
            .unwrap();
        agent.set_root_key(root_key.clone());
        let intent = prepare_intent(
            OperatorMintAuthority {
                operation_id: [1; 32],
                plan_sha256: [2; 32],
                funding_review_sha256: [3; 32],
                network_identity_sha256: network_identity_sha256(&root_key),
                operator: Principal::from_slice(&[1, 1]),
                icp_ledger: Principal::from_slice(&[2, 1]),
                cmc: Principal::from_slice(&[3, 1]),
                cycles_ledger: Principal::from_slice(&[4, 1]),
            },
            100_000_000,
            10_000,
            1,
        )
        .unwrap();
        // Exact pinned Cycles Ledger mint schema: tx fee differs from block fee.
        let block = map([
            (
                "tx",
                map([
                    ("op", Value::Text("mint".into())),
                    (
                        "to",
                        Value::Array(vec![blob(intent.authority.operator.as_slice())]),
                    ),
                    ("memo", blob(&intent.deposit_memo)),
                    ("amt", Value::Nat(1_000_u128.into())),
                    ("fee", Value::Nat(100_u128.into())),
                ]),
            ),
            ("ts", Value::Nat(1_u64.into())),
            ("fee", Value::Nat(0_u64.into())),
        ]);
        let deposit_hash = block.hash();
        let mut tip = block.clone();
        fields(&mut tip).insert("phash".into(), blob(&deposit_hash));
        fields(fields(&mut tip).get_mut("tx").unwrap()).insert("memo".into(), blob(&[9; 32]));
        Self {
            key,
            agent,
            intent,
            outcome: OperatorMintNotificationOutcomeRecord::Minted {
                deposit_block_index: 0,
                gross_minted_cycles: 1_100,
                historical_balance_cycles: u128::MAX,
            },
            chain: vec![
                CyclesLedgerBlock {
                    id: 1_u64.into(),
                    block: tip,
                },
                CyclesLedgerBlock {
                    id: 0_u64.into(),
                    block,
                },
            ],
        }
    }

    fn witness(&self) -> HashTree {
        fork(
            labeled(b"last_block_hash", leaf(self.chain[0].block.hash())),
            labeled(
                b"last_block_index",
                leaf(leb128((&self.chain[0].id.0).try_into().unwrap())),
            ),
        )
    }

    fn certificate(&self, witness: &HashTree, time: u128) -> Certificate {
        sign(
            fork(
                labeled(
                    b"canister",
                    labeled(
                        self.intent.authority.cycles_ledger.as_slice(),
                        labeled(b"certified_data", leaf(witness.digest())),
                    ),
                ),
                labeled(b"time", leaf(leb128(time))),
            ),
            &self.key,
        )
    }

    fn evidence(&self) -> ICRC3DataCertificate {
        let witness = self.witness();
        ICRC3DataCertificate {
            certificate: cbor(&self.certificate(&witness, now())).into(),
            hash_tree: cbor(&witness).into(),
        }
    }

    fn verify(&self, evidence: &ICRC3DataCertificate) -> Result<VerifiedCyclesDeposit, Error> {
        verify_cycles_deposit(
            &self.agent,
            &self.intent,
            &self.outcome,
            evidence,
            &self.chain,
            limits(),
        )
    }

    fn change_deposit(&mut self, change: impl FnOnce(&mut Value)) {
        change(&mut self.chain[1].block);
        let hash = self.chain[1].block.hash();
        fields(&mut self.chain[0].block).insert("phash".into(), blob(&hash));
    }
}

fn limits() -> ReceiptVerificationLimits {
    ReceiptVerificationLimits {
        certificate_bytes: 16_384,
        blocks: 8,
        value_nodes: 128,
        value_bytes: 4_096,
        depth: 32,
    }
}

fn map<const N: usize>(pairs: [(&str, Value); N]) -> Value {
    Value::Map(
        pairs
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .collect(),
    )
}

fn fields(value: &mut Value) -> &mut std::collections::BTreeMap<String, Value> {
    let Value::Map(fields) = value else {
        panic!("map fixture")
    };
    fields
}

fn blob(bytes: &[u8]) -> Value {
    Value::Blob(bytes.to_vec().into())
}

#[test]
fn authenticated_deposit_uses_net_credit_and_transaction_fee_not_historical_balance() {
    let fixture = Fixture::new();
    let result = fixture.verify(&fixture.evidence()).unwrap();
    assert_eq!(result.intent(), &fixture.intent);
    assert_eq!(result.block_index(), 0);
    assert_eq!(result.net_credit_cycles(), 1_000);
    assert_eq!(result.deposit_fee_cycles(), 100);
}

#[test]
fn authenticated_certificate_accepts_owned_pruned_nodes() {
    let fixture = Fixture::new();
    let witness = fixture.witness();
    let certificate = fixture.certificate(&witness, now());
    let certificate = sign(
        fork(ic_certification::pruned([7; 32]), certificate.tree),
        &fixture.key,
    );
    let evidence = ICRC3DataCertificate {
        certificate: cbor(&certificate).into(),
        hash_tree: cbor(&witness).into(),
    };
    assert_eq!(fixture.verify(&evidence).unwrap().net_credit_cycles(), 1000);
}

#[test]
fn changed_root_key_or_ledger_cannot_authenticate_deposit() {
    let mut fixture = Fixture::new();
    let evidence = fixture.evidence();
    fixture.agent.set_root_key(der_key(&signing_key()));
    assert!(matches!(
        fixture.verify(&evidence),
        Err(Error::NetworkMismatch)
    ));
    fixture.agent.set_root_key(der_key(&fixture.key));
    let mut authority = fixture.intent.authority.clone();
    authority.cycles_ledger = Principal::from_slice(&[8, 1]);
    fixture.intent = prepare_intent(authority, 100_000_000, 10_000, 1).unwrap();
    assert!(matches!(
        fixture.verify(&evidence),
        Err(Error::WitnessMismatch)
    ));
}

#[test]
fn signature_expiry_and_witness_tampering_reject() {
    let fixture = Fixture::new();
    let witness = fixture.witness();
    let mut cert = fixture.certificate(&witness, now());
    cert.signature[0] ^= 1;
    let mut evidence = fixture.evidence();
    evidence.certificate = cbor(&cert).into();
    assert!(matches!(
        fixture.verify(&evidence),
        Err(Error::Certificate(CertificateVerificationError::Invalid(_)))
    ));
    evidence.certificate = cbor(&fixture.certificate(&witness, 0)).into();
    assert!(matches!(
        fixture.verify(&evidence),
        Err(Error::Certificate(CertificateVerificationError::Invalid(_)))
    ));
    evidence = fixture.evidence();
    evidence.hash_tree = cbor(&labeled(b"last_block_hash", leaf([0; 32]))).into();
    assert!(matches!(
        fixture.verify(&evidence),
        Err(Error::WitnessMismatch)
    ));
}

#[test]
fn delegated_certificate_requires_the_exact_canister_range() {
    let fixture = Fixture::new();
    let subnet_key = signing_key();
    let subnet = Principal::from_slice(&[7, 1]);
    for included in [true, false] {
        let canister = if included {
            fixture.intent.authority.cycles_ledger
        } else {
            subnet
        };
        let ranges = cbor(&vec![(canister, canister)]);
        let delegation = sign(
            fork(
                labeled(
                    b"canister_ranges",
                    labeled(
                        subnet.as_slice(),
                        labeled(canister.as_slice(), leaf(ranges)),
                    ),
                ),
                labeled(
                    b"subnet",
                    labeled(
                        subnet.as_slice(),
                        labeled(b"public_key", leaf(der_key(&subnet_key))),
                    ),
                ),
            ),
            &fixture.key,
        );
        let mut cert = sign(
            fixture.certificate(&fixture.witness(), now()).tree,
            &subnet_key,
        );
        cert.delegation = Some(Delegation {
            subnet_id: subnet.as_slice().to_vec(),
            certificate: cbor(&delegation),
        });
        let mut evidence = fixture.evidence();
        evidence.certificate = cbor(&cert).into();
        if included {
            fixture.verify(&evidence).unwrap();
        } else {
            assert!(matches!(
                fixture.verify(&evidence),
                Err(Error::Certificate(CertificateVerificationError::Invalid(_)))
            ));
        }
    }
}

#[test]
fn chain_tampering_gaps_duplicate_indices_and_wrong_locator_reject() {
    let mut fixture = Fixture::new();
    let evidence = fixture.evidence();
    fields(&mut fixture.chain[1].block).insert("ts".into(), Value::Nat(2_u64.into()));
    assert!(matches!(
        fixture.verify(&evidence),
        Err(Error::ChainMismatch)
    ));
    fixture = Fixture::new();
    fixture.chain[1].id = 1_u64.into();
    assert!(matches!(
        fixture.verify(&fixture.evidence()),
        Err(Error::ChainMismatch)
    ));
    fixture = Fixture::new();
    fixture.chain.pop();
    assert!(matches!(
        fixture.verify(&fixture.evidence()),
        Err(Error::ChainMismatch)
    ));
    fixture = Fixture::new();
    fixture.outcome = OperatorMintNotificationOutcomeRecord::Minted {
        deposit_block_index: 2,
        gross_minted_cycles: 1_100,
        historical_balance_cycles: 1_000,
    };
    assert!(matches!(
        fixture.verify(&fixture.evidence()),
        Err(Error::ChainMismatch)
    ));
}

#[test]
fn certified_wrong_account_subaccount_or_memo_stays_uncredited() {
    for (field, value) in [
        ("to", Value::Array(vec![blob(&[8, 1])])),
        ("to", Value::Array(vec![blob(&[1, 1]), blob(&[0; 32])])),
        ("memo", blob(&[8; 32])),
    ] {
        let mut fixture = Fixture::new();
        fixture.change_deposit(|block| {
            fields(fields(block).get_mut("tx").unwrap()).insert(field.into(), value);
        });
        assert!(matches!(
            fixture.verify(&fixture.evidence()),
            Err(Error::DepositBindingMismatch)
        ));
    }
}

#[test]
fn certified_unknown_schema_or_missing_deposit_fee_rejects() {
    for field in ["fee", "op"] {
        let mut fixture = Fixture::new();
        fixture.change_deposit(|block| {
            fields(fields(block).get_mut("tx").unwrap()).remove(field);
        });
        assert!(matches!(
            fixture.verify(&fixture.evidence()),
            Err(Error::UnsupportedDeposit)
        ));
    }
    let mut fixture = Fixture::new();
    fixture.change_deposit(|block| {
        fields(block).insert("fee".into(), Value::Nat(100_u64.into()));
    });
    assert!(matches!(
        fixture.verify(&fixture.evidence()),
        Err(Error::UnsupportedDeposit)
    ));
    fixture = Fixture::new();
    fixture.change_deposit(|block| {
        fields(fields(block).get_mut("tx").unwrap())
            .insert("extra".into(), Value::Nat(0_u64.into()));
    });
    assert!(matches!(
        fixture.verify(&fixture.evidence()),
        Err(Error::UnsupportedDeposit)
    ));
}

#[test]
fn certified_zero_overflow_or_gross_mismatch_rejects() {
    for amount in [0, 1_100, u128::MAX] {
        let mut fixture = Fixture::new();
        fixture.change_deposit(|block| {
            fields(fields(block).get_mut("tx").unwrap())
                .insert("amt".into(), Value::Nat(amount.into()));
        });
        assert!(matches!(
            fixture.verify(&fixture.evidence()),
            Err(Error::AmountMismatch)
        ));
    }
}

#[test]
fn verification_has_explicit_budgets_before_hashing() {
    let fixture = Fixture::new();
    for bounds in [
        ReceiptVerificationLimits {
            certificate_bytes: 0,
            ..limits()
        },
        ReceiptVerificationLimits {
            blocks: 1,
            ..limits()
        },
        ReceiptVerificationLimits {
            value_nodes: 1,
            ..limits()
        },
        ReceiptVerificationLimits {
            value_bytes: 1,
            ..limits()
        },
        ReceiptVerificationLimits {
            depth: 1,
            ..limits()
        },
    ] {
        assert!(matches!(
            verify_cycles_deposit(
                &fixture.agent,
                &fixture.intent,
                &fixture.outcome,
                &fixture.evidence(),
                &fixture.chain,
                bounds,
            ),
            Err(Error::BudgetExceeded)
        ));
    }
}

#[test]
fn malformed_or_trailing_cbor_and_noncanonical_tip_index_reject() {
    let fixture = Fixture::new();
    let mut evidence = fixture.evidence();
    evidence.certificate.push(0);
    assert!(matches!(
        fixture.verify(&evidence),
        Err(Error::Certificate(CertificateVerificationError::Malformed))
    ));
    evidence = fixture.evidence();
    evidence.hash_tree = vec![0xff].into();
    assert!(matches!(
        fixture.verify(&evidence),
        Err(Error::Certificate(CertificateVerificationError::Decode(_)))
    ));
    for index in [vec![], vec![0x81, 0], vec![0x80], vec![0xff; 20]] {
        let witness = fork(
            labeled(b"last_block_hash", leaf(fixture.chain[0].block.hash())),
            labeled(b"last_block_index", leaf(index)),
        );
        evidence = ICRC3DataCertificate {
            certificate: cbor(&fixture.certificate(&witness, now())).into(),
            hash_tree: cbor(&witness).into(),
        };
        assert!(matches!(
            fixture.verify(&evidence),
            Err(Error::WitnessMismatch)
        ));
    }
}

#[test]
fn unresolved_notification_is_never_a_deposit_proof() {
    let mut fixture = Fixture::new();
    fixture.outcome = OperatorMintNotificationOutcomeRecord::Processing;
    assert!(matches!(
        fixture.verify(&fixture.evidence()),
        Err(Error::MissingMintLocator)
    ));
}

#[test]
fn oversized_signed_values_reject_before_the_upstream_hasher_can_panic() {
    let mut fixture = Fixture::new();
    let evidence = fixture.evidence();
    fields(&mut fixture.chain[0].block)
        .insert("extra".into(), Value::Int(candid::Int(u128::MAX.into())));
    assert!(matches!(
        fixture.verify(&evidence),
        Err(Error::UnsupportedBlockValue)
    ));
}

#[test]
fn authenticated_indices_and_credit_preserve_the_full_accounting_width() {
    let mut fixture = Fixture::new();
    fixture.chain[0].id = u128::MAX.into();
    fixture.chain[1].id = (u128::MAX - 1).into();
    fixture.change_deposit(|block| {
        fields(block).insert("phash".into(), blob(&[17; 32]));
        fields(fields(block).get_mut("tx").unwrap())
            .insert("amt".into(), Value::Nat((u128::MAX - 100).into()));
    });
    fixture.outcome = OperatorMintNotificationOutcomeRecord::Minted {
        deposit_block_index: u128::MAX - 1,
        gross_minted_cycles: u128::MAX,
        historical_balance_cycles: u128::MAX,
    };
    let result = fixture.verify(&fixture.evidence()).unwrap();
    assert_eq!(result.block_index(), u128::MAX - 1);
    assert_eq!(result.net_credit_cycles(), u128::MAX - 100);
}

#[test]
fn block_wire_projection_matches_the_published_ledger_value_contract() {
    // Pinned Cycles Ledger DID at b1ea7da, including its Nat64 variant.
    let contract = r"
        type Value = variant {
            Int : int; Map : vec record { text; Value }; Nat : nat; Nat64 : nat64;
            Blob : vec nat8; Text : text; Array : vec Value;
        };
        type Block = record { id : nat; block : Value };
    ";
    let (mut env, _) = candid_parser::utils::CandidSource::Text(contract)
        .load()
        .unwrap();
    let expected = env.find_type("Block").unwrap().clone();
    let mut rust = candid::types::internal::TypeContainer::new();
    let ty = rust.add::<CyclesLedgerBlock>();
    let actual = env.merge_type(rust.env, ty);
    candid::types::subtype::equal(
        &mut std::collections::HashSet::new(),
        &env,
        &expected,
        &actual,
    )
    .unwrap();
}
