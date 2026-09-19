//! Certified Ledger delegation and exact replicated archive receipt regressions.

use super::{Fixture, limits};
use crate::fleet_ensure::ops::operator_mint::receipts::{
    CertificateVerificationError,
    icp::{
        ArchiveReadError, IcpArchiveAuthorization, IcpReadError as Error, IcpReadOutcome,
        IcpTransferRead, prepare_archive_read, verify_read,
        wire::{ArchivedRange, EncodedBlocksReply, ReadArgs},
    },
    test_support::{cbor, der_key, sign, signing_key},
};
use candid::Principal;
use ic_agent::agent::signed_update_inspect;
use ic_certification::{Certificate, Delegation, fork, labeled, leaf};
use icrc_ledger_types::icrc3::archive::QueryArchiveFn;
use serde_bytes::ByteBuf;
use sha2_host::{Digest, Sha256};

fn callback() -> ArchivedRange {
    ArchivedRange {
        callback: QueryArchiveFn::new(Principal::from_slice(&[6, 1]), "get_encoded_blocks"),
        start: 77,
        length: 1,
    }
}

fn redirect(fixture: &Fixture) -> EncodedBlocksReply {
    EncodedBlocksReply {
        // The Ledger explicitly leaves this field unspecified without local blocks.
        first_block_index: 90,
        blocks: vec![],
        archived_blocks: vec![callback()],
        ..fixture.reply()
    }
}

fn authorization(fixture: &Fixture) -> IcpArchiveAuthorization {
    let IcpReadOutcome::Archive(archive) = verify_read(
        &fixture.agent,
        &fixture.read,
        &fixture.evidence(&redirect(fixture)),
        limits(),
    )
    .unwrap() else {
        panic!("expected archive authorization")
    };
    archive
}

fn evidence(
    fixture: &Fixture,
    read: &IcpTransferRead,
    reply: Result<Vec<ByteBuf>, ArchiveReadError>,
) -> Vec<u8> {
    fixture.certificate(
        read.request_id(),
        b"replied",
        &candid::encode_one(reply).unwrap(),
    )
}

#[test]
fn certified_redirect_and_archive_reply_authenticate_the_original_transfer() {
    let fixture = Fixture::new();
    let archive = authorization(&fixture);
    assert_eq!(archive.canister(), callback().callback.canister_id);
    let read = prepare_archive_read(&fixture.agent, &archive).unwrap();
    signed_update_inspect(
        fixture.intent.authority.operator,
        archive.canister(),
        "get_encoded_blocks",
        &candid::encode_one(ReadArgs {
            start: 77,
            length: 1,
        })
        .unwrap(),
        read.request.ingress_expiry,
        read.signed_request().to_vec(),
    )
    .unwrap();
    assert_ne!(read.request_id(), fixture.read.request_id());
    let cert = evidence(&fixture, &read, Ok(vec![fixture.block.clone().into()]));
    let IcpReadOutcome::Transfer(transfer) =
        verify_read(&fixture.agent, &read, &cert, limits()).unwrap()
    else {
        panic!("expected verified archived transfer")
    };
    assert_eq!(transfer.intent(), &fixture.intent);
    assert_eq!(transfer.block_index(), 77);
    assert_eq!(
        *transfer.block_sha256(),
        <[u8; 32]>::from(Sha256::digest(&fixture.block))
    );
}

#[test]
fn redirect_must_name_exactly_one_supported_archive_block() {
    let fixture = Fixture::new();
    for case in 0..9 {
        let mut reply = redirect(&fixture);
        let archive = &mut reply.archived_blocks[0];
        match case {
            0 => archive.start += 1,
            1 => archive.length = 0,
            2 => archive.length = u64::MAX,
            3 => archive.callback.method = "transfer".into(),
            4 => archive.callback.canister_id = Principal::anonymous(),
            5 => archive.callback.canister_id = Principal::management_canister(),
            6 => archive.callback.canister_id = fixture.intent.authority.icp_ledger,
            7 => reply.archived_blocks.push(callback()),
            _ => reply.blocks.push(fixture.block.clone().into()),
        }
        assert!(matches!(
            verify_read(
                &fixture.agent,
                &fixture.read,
                &fixture.evidence(&reply),
                limits()
            ),
            Err(Error::InvalidArchive)
        ));
    }
}

#[test]
fn archive_signing_rejects_changed_network_or_operator() {
    let fixture = Fixture::new();
    let archive = authorization(&fixture);
    let other = Fixture::new();
    other.agent.set_root_key(der_key(&fixture.key));
    assert!(matches!(
        prepare_archive_read(&other.agent, &archive),
        Err(Error::ReaderMismatch)
    ));
    fixture.agent.set_root_key(der_key(&other.key));
    assert!(matches!(
        prepare_archive_read(&fixture.agent, &archive),
        Err(Error::ReaderMismatch)
    ));
}

#[test]
fn archive_certificate_must_authorize_archive_canister_not_ledger() {
    let fixture = Fixture::new();
    let archive = authorization(&fixture);
    let read = prepare_archive_read(&fixture.agent, &archive).unwrap();
    let cert = evidence(&fixture, &read, Ok(vec![fixture.block.clone().into()]));
    let original: Certificate = ciborium::de::from_reader(cert.as_slice()).unwrap();
    let subnet_key = signing_key();
    let subnet = Principal::from_slice(&[7, 1]);
    for canister in [archive.canister(), fixture.intent.authority.icp_ledger] {
        let delegation = sign(
            fork(
                labeled(
                    b"canister_ranges",
                    labeled(
                        subnet.as_slice(),
                        labeled(canister.as_slice(), leaf(cbor(&vec![(canister, canister)]))),
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
        let mut certificate = sign(original.tree.clone(), &subnet_key);
        certificate.delegation = Some(Delegation {
            subnet_id: subnet.as_slice().to_vec(),
            certificate: cbor(&delegation),
        });
        let result = verify_read(&fixture.agent, &read, &cbor(&certificate), limits());
        if canister == archive.canister() {
            assert!(matches!(result, Ok(IcpReadOutcome::Transfer(_))));
        } else {
            assert!(matches!(
                result,
                Err(Error::Certificate(CertificateVerificationError::Invalid(_)))
            ));
        }
    }
}

#[test]
fn archive_errors_and_missing_or_extra_blocks_cannot_prove_payment() {
    let fixture = Fixture::new();
    let read = prepare_archive_read(&fixture.agent, &authorization(&fixture)).unwrap();
    for blocks in [vec![], vec![fixture.block.clone().into(); 2]] {
        assert!(matches!(
            verify_read(
                &fixture.agent,
                &read,
                &evidence(&fixture, &read, Ok(blocks)),
                limits()
            ),
            Err(Error::BlockMismatch)
        ));
    }
    let cert = evidence(
        &fixture,
        &read,
        Err(ArchiveReadError::BadFirstBlockIndex {
            requested_index: 77,
            first_valid_index: 80,
        }),
    );
    assert!(matches!(
        verify_read(&fixture.agent, &read, &cert, limits()),
        Err(Error::Archive(ArchiveReadError::BadFirstBlockIndex {
            requested_index: 77,
            first_valid_index: 80
        }))
    ));
    let cert = evidence(
        &fixture,
        &read,
        Err(ArchiveReadError::Other {
            error_code: 42,
            error_message: "unavailable".into(),
        }),
    );
    assert!(matches!(
        verify_read(&fixture.agent, &read, &cert, limits()),
        Err(Error::Archive(ArchiveReadError::Other {
            error_code: 42,
            ..
        }))
    ));
}

#[test]
fn archive_cannot_reuse_ledger_reply_redirect_again_or_change_transfer() {
    let mut fixture = Fixture::new();
    let read = prepare_archive_read(&fixture.agent, &authorization(&fixture)).unwrap();
    let reply = redirect(&fixture);
    assert!(matches!(
        verify_read(&fixture.agent, &read, &fixture.evidence(&reply), limits()),
        Err(Error::ReplyUnavailable)
    ));
    let cert = fixture.certificate(
        read.request_id(),
        b"replied",
        &candid::encode_one(reply).unwrap(),
    );
    assert!(matches!(
        verify_read(&fixture.agent, &read, &cert, limits()),
        Err(Error::InvalidReply)
    ));
    fixture.altered_block(|block| {
        block
            .transaction
            .as_mut()
            .unwrap()
            .send
            .as_mut()
            .unwrap()
            .amount
            .as_mut()
            .unwrap()
            .value += 1;
    });
    let cert = evidence(&fixture, &read, Ok(vec![fixture.block.clone().into()]));
    assert!(matches!(
        verify_read(&fixture.agent, &read, &cert, limits()),
        Err(Error::TransferMismatch)
    ));
}
