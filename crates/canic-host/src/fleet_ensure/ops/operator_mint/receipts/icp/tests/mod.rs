//! Exact replicated read identity and authenticated protobuf receipt regressions.

mod archive;

use super::{
    EncodedBlocksReply, IcpReadError as Error, IcpReadLimits, IcpReadOutcome, IcpTransferRead,
    prepare_read, protobuf, verify_read,
};
use crate::fleet_ensure::{
    model::operator_mint::{OperatorMintAuthority, OperatorMintIntentRecord},
    ops::operator_mint::{
        account_identifier, prepare_intent, principal_subaccount,
        receipts::{
            CertificateVerificationError, network_identity_sha256,
            test_support::{cbor, der_key, leb128, now, sign, signing_key},
        },
    },
};
use candid::{CandidType, Principal};
use ic_agent::{
    Agent, RequestId,
    agent::{EnvelopeContent, signed_update_inspect},
    identity::BasicIdentity,
};
use ic_certification::{fork, labeled, leaf};
use ic_verify_bls_signature::PrivateKey;
use prost::Message;
use serde::Deserialize;
use sha2_host::{Digest, Sha256};

struct Fixture {
    key: PrivateKey,
    agent: Agent,
    intent: OperatorMintIntentRecord,
    read: IcpTransferRead,
    block: Vec<u8>,
}

#[derive(CandidType, Deserialize)]
struct Args {
    start: u64,
    length: u64,
}

impl Fixture {
    fn new() -> Self {
        let key = signing_key();
        let mut identity_bytes = [0; 32];
        getrandom::fill(&mut identity_bytes).unwrap();
        let agent = Agent::builder()
            .with_url("http://127.0.0.1:1")
            .with_identity(BasicIdentity::from_raw_key(&identity_bytes))
            .build()
            .unwrap();
        let root = der_key(&key);
        agent.set_root_key(root.clone());
        let intent = prepare_intent(
            OperatorMintAuthority {
                operation_id: [1; 32],
                plan_sha256: [2; 32],
                funding_review_sha256: [3; 32],
                network_identity_sha256: network_identity_sha256(&root),
                operator: agent.get_principal().unwrap(),
                icp_ledger: Principal::from_slice(&[2, 1]),
                cmc: Principal::from_slice(&[3, 1]),
                cycles_ledger: Principal::from_slice(&[4, 1]),
            },
            100_000_000,
            10_000,
            123,
        )
        .unwrap();
        let read = prepare_read(&agent, &intent, 77).unwrap();
        // Independent field encoding from the pinned types.proto, without using
        // the production projection's derives to create the acceptance fixture.
        let from = account_identifier(intent.authority.operator, &[0; 32]);
        let to = account_identifier(
            intent.authority.cmc,
            &principal_subaccount(intent.authority.operator),
        );
        let send = [
            message(1, &message(1, &from)),
            message(2, &message(1, &to)),
            message(3, &unsigned(100_000_000)),
            message(4, &unsigned(10_000)),
        ]
        .concat();
        let tx = [
            message(3, &send),
            message(4, &unsigned(0x544e_494d)),
            message(6, &unsigned(123)),
        ]
        .concat();
        let block = [
            message(1, &message(1, &[9; 32])),
            message(2, &unsigned(456)),
            message(3, &tx),
        ]
        .concat();
        Self {
            key,
            agent,
            intent,
            read,
            block,
        }
    }

    fn reply(&self) -> EncodedBlocksReply {
        EncodedBlocksReply {
            chain_length: 90,
            first_block_index: 77,
            blocks: vec![self.block.clone().into()],
            archived_blocks: vec![],
        }
    }

    fn certificate(&self, request: &RequestId, status: &[u8], reply: &[u8]) -> Vec<u8> {
        cbor(&sign(
            fork(
                labeled(
                    b"request_status",
                    labeled(
                        request.as_slice(),
                        fork(
                            labeled(b"reply", leaf(reply)),
                            labeled(b"status", leaf(status)),
                        ),
                    ),
                ),
                labeled(b"time", leaf(leb128(now()))),
            ),
            &self.key,
        ))
    }

    fn evidence(&self, reply: &EncodedBlocksReply) -> Vec<u8> {
        self.certificate(
            self.read.request_id(),
            b"replied",
            &candid::encode_one(reply).unwrap(),
        )
    }

    fn altered_block(&mut self, change: impl FnOnce(&mut protobuf::Block)) {
        let mut block = protobuf::Block::decode(self.block.as_slice()).unwrap();
        change(&mut block);
        self.block = block.encode_to_vec();
    }
}

fn message(tag: u8, value: &[u8]) -> Vec<u8> {
    [
        vec![(tag << 3) | 2],
        leb128(value.len() as u128),
        value.to_vec(),
    ]
    .concat()
}

fn unsigned(value: u128) -> Vec<u8> {
    [vec![8], leb128(value)].concat()
}

const fn limits() -> IcpReadLimits {
    IcpReadLimits {
        certificate_bytes: 16_384,
        certificate_depth: 32,
        reply_bytes: 8_192,
        decoding_quota: 100_000,
        skipping_quota: 8_192,
    }
}

#[test]
fn signed_read_binds_operator_ledger_method_and_single_block_index() {
    let fixture = Fixture::new();
    let arg = candid::encode_one(Args {
        start: 77,
        length: 1,
    })
    .unwrap();
    let request = &fixture.read.request;
    signed_update_inspect(
        fixture.intent.authority.operator,
        fixture.intent.authority.icp_ledger,
        "query_encoded_blocks",
        &arg,
        request.ingress_expiry,
        fixture.read.signed_request().to_vec(),
    )
    .unwrap();
    let content = EnvelopeContent::Call {
        sender: fixture.intent.authority.operator,
        canister_id: fixture.intent.authority.icp_ledger,
        method_name: "query_encoded_blocks".into(),
        arg,
        ingress_expiry: request.ingress_expiry,
        nonce: request.nonce.clone(),
        sender_info: request.sender_info.clone(),
    };
    assert_eq!(
        ic_agent::to_request_id(&content).unwrap(),
        *fixture.read.request_id()
    );
    let IcpReadOutcome::Transfer(result) = verify_read(
        &fixture.agent,
        &fixture.read,
        &fixture.evidence(&fixture.reply()),
        limits(),
    )
    .unwrap() else {
        panic!("expected a verified local transfer")
    };
    assert_eq!(result.intent(), &fixture.intent);
    assert_eq!(result.block_index(), 77);
    assert_eq!(
        *result.block_sha256(),
        <[u8; 32]>::from(Sha256::digest(&fixture.block))
    );
}

#[test]
fn certificate_for_a_different_read_cannot_authorize_this_block() {
    let fixture = Fixture::new();
    let other = prepare_read(&fixture.agent, &fixture.intent, 78).unwrap();
    let certificate = fixture.certificate(
        other.request_id(),
        b"replied",
        &candid::encode_one(fixture.reply()).unwrap(),
    );
    assert!(matches!(
        verify_read(&fixture.agent, &fixture.read, &certificate, limits()),
        Err(Error::ReplyUnavailable)
    ));
}

#[test]
fn pending_rejected_and_deleted_read_states_never_become_transfer_proofs() {
    let fixture = Fixture::new();
    for status in [b"processing".as_slice(), b"rejected", b"done"] {
        let cert = fixture.certificate(
            fixture.read.request_id(),
            status,
            &candid::encode_one(fixture.reply()).unwrap(),
        );
        assert!(matches!(
            verify_read(&fixture.agent, &fixture.read, &cert, limits()),
            Err(Error::ReplyUnavailable)
        ));
    }
}

#[test]
fn changed_network_signer_and_certificate_signature_reject() {
    let fixture = Fixture::new();
    let cert = fixture.evidence(&fixture.reply());
    fixture.agent.set_root_key(der_key(&signing_key()));
    assert!(matches!(
        prepare_read(&fixture.agent, &fixture.intent, 77),
        Err(Error::ReaderMismatch)
    ));
    assert!(matches!(
        verify_read(&fixture.agent, &fixture.read, &cert, limits()),
        Err(Error::ReaderMismatch)
    ));
    fixture.agent.set_root_key(der_key(&fixture.key));
    let other = Fixture::new();
    other.agent.set_root_key(der_key(&fixture.key));
    assert!(matches!(
        verify_read(&other.agent, &fixture.read, &cert, limits()),
        Err(Error::ReaderMismatch)
    ));
    let mut certificate: ic_certification::Certificate =
        ciborium::de::from_reader(cert.as_slice()).unwrap();
    certificate.signature[0] ^= 1;
    assert!(matches!(
        verify_read(&fixture.agent, &fixture.read, &cbor(&certificate), limits()),
        Err(Error::Certificate(CertificateVerificationError::Invalid(_)))
    ));
}

#[test]
fn certificate_and_reply_decoding_are_explicitly_bounded() {
    let fixture = Fixture::new();
    let certificate = fixture.evidence(&fixture.reply());
    assert!(matches!(
        verify_read(
            &fixture.agent,
            &fixture.read,
            &certificate,
            IcpReadLimits {
                certificate_bytes: 0,
                ..limits()
            }
        ),
        Err(Error::Certificate(
            CertificateVerificationError::BudgetExceeded
        ))
    ));
    assert!(matches!(
        verify_read(
            &fixture.agent,
            &fixture.read,
            &certificate,
            IcpReadLimits {
                reply_bytes: 0,
                ..limits()
            }
        ),
        Err(Error::BudgetExceeded)
    ));
    assert!(matches!(
        verify_read(
            &fixture.agent,
            &fixture.read,
            &certificate,
            IcpReadLimits {
                decoding_quota: 0,
                ..limits()
            }
        ),
        Err(Error::InvalidReply)
    ));
}

#[test]
fn missing_blocks_wrong_indices_and_short_chains_stay_unresolved() {
    let fixture = Fixture::new();
    for reply in [
        EncodedBlocksReply {
            blocks: vec![],
            ..fixture.reply()
        },
        EncodedBlocksReply {
            first_block_index: 78,
            ..fixture.reply()
        },
        EncodedBlocksReply {
            chain_length: 77,
            ..fixture.reply()
        },
    ] {
        assert!(matches!(
            verify_read(
                &fixture.agent,
                &fixture.read,
                &fixture.evidence(&reply),
                limits()
            ),
            Err(Error::BlockMismatch)
        ));
    }
}

#[test]
fn every_transfer_binding_is_checked_against_the_retained_intent() {
    for field in 0..6 {
        let mut fixture = Fixture::new();
        fixture.altered_block(|block| {
            let tx = block.transaction.as_mut().unwrap();
            let send = tx.send.as_mut().unwrap();
            match field {
                0 => send.from.as_mut().unwrap().value[0] ^= 1,
                1 => send.to.as_mut().unwrap().value[0] ^= 1,
                2 => send.amount.as_mut().unwrap().value += 1,
                3 => send.fee.as_mut().unwrap().value += 1,
                4 => tx.memo.as_mut().unwrap().value += 1,
                _ => tx.created_at_time.as_mut().unwrap().value += 1,
            }
        });
        assert!(matches!(
            verify_read(
                &fixture.agent,
                &fixture.read,
                &fixture.evidence(&fixture.reply()),
                limits()
            ),
            Err(Error::TransferMismatch)
        ));
    }
}

#[test]
fn missing_fee_or_creation_time_never_uses_ledger_view_defaults() {
    for remove_fee in [true, false] {
        let mut fixture = Fixture::new();
        fixture.altered_block(|block| {
            // The decoded Candid view would substitute this timestamp.
            block.timestamp.as_mut().unwrap().value = 123;
            let tx = block.transaction.as_mut().unwrap();
            if remove_fee {
                tx.send.as_mut().unwrap().fee = None;
            } else {
                tx.created_at_time = None;
            }
        });
        assert!(matches!(
            verify_read(
                &fixture.agent,
                &fixture.read,
                &fixture.evidence(&fixture.reply()),
                limits()
            ),
            Err(Error::UnsupportedBlock)
        ));
    }
}

#[test]
fn unknown_and_duplicate_protobuf_fields_do_not_disappear_during_decode() {
    for extra in [message(4, &[]), message(2, &unsigned(456))] {
        let mut fixture = Fixture::new();
        fixture.block.extend(extra);
        assert!(matches!(
            verify_read(
                &fixture.agent,
                &fixture.read,
                &fixture.evidence(&fixture.reply()),
                limits()
            ),
            Err(Error::UnsupportedBlock)
        ));
    }
}
