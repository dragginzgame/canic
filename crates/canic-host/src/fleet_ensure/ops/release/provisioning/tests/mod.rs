//! Complete discovery, bounded decoding and exact cursor/header refusal proofs.

use super::*;
use crate::fleet_ensure::{
    view::release::provisioning::{ReleaseProvisioningDisposition, ReleaseProvisioningOwner},
    workflow::release::assess_provisioning_evidence,
};
use candid::CandidType;
use candid::types::{Field, Label, Type, TypeEnv, TypeInner};
use canic_contracts::dto::{
    component_provisioning::{
        ProvisioningFailureStage, ProvisioningRetryCategory, RootComponentProvisioningFailure,
    },
    root::RootProvisioningReleaseEntry,
};

pub(in crate::fleet_ensure::ops::release) fn fixture(
    root: Principal,
) -> RootProvisioningReleaseResponse {
    RootProvisioningReleaseResponse {
        root,
        active_provisioning: None,
        active_directory_synchronization: None,
        entry: Some(RootProvisioningReleaseEntry {
            key: Key::Provisioning([7; 32]),
            plan_hash: [8; 32],
            phase: Phase::Accepted,
            delivery_in_flight: None,
            last_failure: None,
        }),
        next_after: None,
    }
}

pub(in crate::fleet_ensure::ops::release) fn wire(
    page: RootProvisioningReleaseResponse,
) -> Vec<u8> {
    candid::encode_one(Ok::<_, CanicError>(Response::ProvisioningRelease(page))).unwrap()
}

fn decode_with_budget(
    root: Principal,
    bytes: &[u8],
) -> Result<RootProvisioningReleaseResponse, ReleaseProvisioningError> {
    let mut remaining = MAXIMUM_CENSUS_BYTES;
    decode_response(root, bytes, &mut remaining)
}

#[test]
fn discovers_both_owners_with_the_same_id_and_preserves_terminal_history() {
    let root = Principal::from_slice(&[1]);
    let mut first = fixture(root);
    first.next_after = Some(first.entry.as_ref().unwrap().key);
    let mut second = fixture(root);
    let entry = second.entry.as_mut().unwrap();
    entry.key = Key::DirectorySynchronization([7; 32]);
    entry.phase = Phase::DirectorySynchronized;
    let mut pages = Pages::new(root);
    assert!(!pages.push(first.clone()).unwrap());
    assert!(pages.push(second.clone()).unwrap());
    assert_eq!(pages.pages, vec![first, second]);
}

#[test]
fn refuses_foreign_owners_changed_headers_and_nonadvancing_or_missing_rows() {
    let root = Principal::from_slice(&[1]);
    let mut first = fixture(root);
    first.next_after = Some(first.entry.as_ref().unwrap().key);
    let mut terminal = fixture(root);
    terminal.entry.as_mut().unwrap().key = Key::Provisioning([9; 32]);
    let mut cases = Vec::new();
    let mut invalid = terminal.clone();
    invalid.root = Principal::anonymous();
    cases.push((invalid, ReleaseProvisioningStage::Binding));
    let mut invalid = terminal.clone();
    invalid.active_provisioning = Some([7; 32]);
    cases.push((invalid, ReleaseProvisioningStage::Binding));
    let mut invalid = terminal.clone();
    invalid.active_directory_synchronization = Some([7; 32]);
    cases.push((invalid, ReleaseProvisioningStage::Binding));
    let mut invalid = terminal.clone();
    invalid.entry.as_mut().unwrap().phase = Phase::DirectoryPlanned;
    cases.push((invalid, ReleaseProvisioningStage::Binding));
    cases.push((fixture(root), ReleaseProvisioningStage::Pagination));
    let mut invalid = terminal.clone();
    invalid.entry = None;
    cases.push((invalid, ReleaseProvisioningStage::Pagination));
    let mut invalid = terminal;
    invalid.next_after = first.next_after;
    cases.push((invalid, ReleaseProvisioningStage::Pagination));
    for (invalid, expected) in cases {
        let mut pages = Pages::new(root);
        assert!(!pages.push(first.clone()).unwrap());
        assert_eq!(pages.push(invalid), Err(expected));
        assert_eq!(pages.pages, vec![first.clone()]);
    }
}

#[test]
fn accepts_exact_record_allowance_but_refuses_a_promised_extra_page() {
    let root = Principal::from_slice(&[1]);
    let mut pages = Pages::new(root);
    for index in 0..MAXIMUM_OPERATIONS {
        let mut id = [0; 32];
        id[..8].copy_from_slice(&u64::try_from(index).unwrap().to_be_bytes());
        let mut page = fixture(root);
        page.entry.as_mut().unwrap().key = Key::Provisioning(id);
        page.next_after = Some(Key::Provisioning(id));
        if index == MAXIMUM_OPERATIONS - 1 {
            assert_eq!(
                pages.push(page.clone()),
                Err(ReleaseProvisioningStage::Budget)
            );
            page.next_after = None;
            assert!(pages.push(page).unwrap());
        } else {
            assert!(!pages.push(page).unwrap());
        }
    }
    assert_eq!(pages.pages.len(), MAXIMUM_OPERATIONS);
}

#[test]
fn decoder_bounds_raw_bytes_total_bytes_and_skipped_work() {
    #[derive(CandidType)]
    enum Extended {
        ProvisioningRelease { root: Principal, extra: Vec<()> },
    }

    let root = Principal::from_slice(&[1]);
    let bytes = wire(fixture(root));
    let mut exhausted = 0;
    assert!(matches!(
        decode_response(root, &bytes, &mut exhausted),
        Err(ReleaseProvisioningError::Observation {
            stage: ReleaseProvisioningStage::Budget,
            ..
        })
    ));
    for bytes in [b"DIDL".to_vec(), vec![0; RESPONSE_BYTES + 1]] {
        assert!(matches!(
            decode_with_budget(root, &bytes),
            Err(ReleaseProvisioningError::Observation {
                stage: ReleaseProvisioningStage::Decode,
                ..
            })
        ));
    }
    let bytes = candid::encode_one(Ok::<_, CanicError>(Extended::ProvisioningRelease {
        root,
        extra: vec![(); RESPONSE_BYTES * 8 + 1],
    }))
    .unwrap();
    assert!(bytes.len() < RESPONSE_BYTES);
    assert!(
        candid::decode_one::<Result<Response, CanicError>>(&bytes)
            .unwrap()
            .is_ok()
    );
    assert!(matches!(
        decode_with_budget(root, &bytes),
        Err(ReleaseProvisioningError::Observation {
            stage: ReleaseProvisioningStage::Decode,
            ..
        })
    ));
}

#[test]
fn empty_census_and_root_refusal_remain_distinct() {
    let root = Principal::from_slice(&[1]);
    let mut empty = fixture(root);
    empty.entry = None;
    assert!(Pages::new(root).push(empty.clone()).unwrap());
    assert_eq!(
        decode_with_budget(root, &wire(empty.clone())).unwrap(),
        empty
    );
    let rejection =
        CanicError::from_registered(canic_contracts::diagnostics::codes::AUTHORITY_UNAVAILABLE);
    let bytes = candid::encode_one(Err::<Response, _>(rejection)).unwrap();
    assert!(matches!(decode_with_budget(root, &bytes),
        Err(ReleaseProvisioningError::Rejected { rejection: observed, .. }) if observed == rejection));
}

#[test]
fn accepts_expanded_reply_type_tables_while_preserving_the_selected_page() {
    let root = Principal::from_slice(&[1]);
    let page = fixture(root);
    let args = candid::IDLArgs::from_bytes(&wire(page.clone())).unwrap();
    let mut extra: Type = TypeInner::Null.into();
    // A full endpoint union carries types for its other variants too. The
    // production Root already has more than 128, even for one compact page.
    for _ in 0..160 {
        extra = TypeInner::Opt(extra).into();
    }
    let TypeInner::Variant(mut fields) = Result::<Response, CanicError>::_ty().as_ref().clone()
    else {
        panic!("Candid result union");
    };
    fields.push(Field {
        id: Label::Named("OtherProtocolReply".into()).into(),
        ty: extra,
    });
    fields.sort_by_key(|field| field.id.get_id());
    let expanded = args
        .to_bytes_with_types(&TypeEnv::new(), &[TypeInner::Variant(fields).into()])
        .unwrap();
    let mut too_small = candid::de::DecoderConfig::new();
    too_small.set_max_type_len(128);
    assert!(
        candid::utils::decode_one_with_config::<Result<Response, CanicError>>(
            &expanded, &too_small
        )
        .is_err()
    );
    assert_eq!(decode_with_budget(root, &expanded).unwrap(), page);
}

#[test]
fn wire_assessment_retains_both_owners_and_ignores_stale_failure_on_completion() {
    let root = Principal::from_slice(&[1]);
    let mut first = fixture(root);
    first.active_provisioning = Some([7; 32]);
    first.active_directory_synchronization = Some([7; 32]);
    let entry = first.entry.as_mut().unwrap();
    entry.phase = Phase::RuntimesActive;
    entry.last_failure = Some(RootComponentProvisioningFailure {
        stage: ProvisioningFailureStage::Provisioning,
        target: root,
        operation_id: [7; 32],
        diagnostic_code: 81,
        retry_category: ProvisioningRetryCategory::ReviewRequired,
        failed_at_ns: 1,
        consecutive_failures: u32::MAX,
        retry_at_ns: None,
    });
    first.next_after = Some(entry.key);
    let recipient = Principal::from_slice(&[2]);
    let mut second = fixture(root);
    second.active_provisioning = first.active_provisioning;
    second.active_directory_synchronization = first.active_directory_synchronization;
    let entry = second.entry.as_mut().unwrap();
    entry.key = Key::DirectorySynchronization([7; 32]);
    entry.phase = Phase::DirectorySynchronizing;
    entry.delivery_in_flight = Some(recipient);
    let originals = vec![first, second];
    let mut pages = Pages::new(root);
    for page in &originals {
        pages
            .push(decode_with_budget(root, &wire(page.clone())).unwrap())
            .unwrap();
    }
    let evidence = FleetReleaseProvisioningView {
        roots: vec![FleetReleaseRootProvisioningView {
            root,
            pages: pages.pages,
        }],
    };
    let assessed = assess_provisioning_evidence(evidence);
    assert_eq!(assessed.evidence.roots[0].pages, originals);
    let result = &assessed.roots[0];
    assert_eq!(result.root, root);
    assert!(result.unmatched_active.is_empty());
    let [provisioning, directory] = result.operations.as_slice() else {
        panic!("two independently owned journals");
    };
    assert_eq!(
        provisioning.facts.identity.owner,
        ReleaseProvisioningOwner::Provisioning
    );
    assert_eq!(
        directory.facts.identity.owner,
        ReleaseProvisioningOwner::DirectorySynchronization
    );
    assert_eq!(
        provisioning.facts.identity.operation_id,
        directory.facts.identity.operation_id
    );
    assert_eq!(provisioning.facts.plan_hash, [8; 32]);
    assert_eq!(directory.facts.plan_hash, [8; 32]);
    assert_eq!(
        provisioning.disposition,
        ReleaseProvisioningDisposition::RecordedCompletion
    );
    assert_eq!(
        directory.disposition,
        ReleaseProvisioningDisposition::DeliveryReconciliation { recipient }
    );
}
