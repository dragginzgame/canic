use super::*;
use crate::{
    diagnostics::codes,
    dto::error::Error,
    dto::fixture_provisioning::{
        FixtureChunkDescriptor, FixtureDescriptor, FixtureGrant, FixtureImportReceipt,
        FixtureTargetBinding,
    },
    ids::{ReleaseBuildId, ReleaseBuildNonce},
    ops::fixture_content,
};
use candid::Principal;
use sha2::{Digest, Sha256};

fn assignment() -> FixtureAssignment {
    let descriptor = FixtureDescriptor {
        schema_version: 1,
        format_hash: [1; 32],
        encoded_length: 3,
        chunks: vec![FixtureChunkDescriptor {
            digest: Sha256::digest(b"row").into(),
            length: 3,
        }],
        completion_summary: [2; 32],
    };
    FixtureAssignment {
        store: Principal::from_slice(&[3; 29]),
        grant: FixtureGrant {
            revision: 1,
            enabled: true,
            binding: FixtureTargetBinding {
                target: crate::test::support::managed_component_binding(),
                installation: [4; 32],
                release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                    [5; 32],
                )),
                content_id: fixture_content::content_id(&descriptor).unwrap(),
            },
        },
        descriptor,
    }
}

fn completed(assignment: &FixtureAssignment) -> FixtureImportProgress {
    FixtureImportProgress {
        binding: assignment.grant.binding.clone(),
        next_chunk: 1,
        receipt: Some(Box::new(FixtureImportReceipt {
            binding: assignment.grant.binding.clone(),
            completion_summary: assignment.descriptor.completion_summary,
        })),
    }
}

#[test]
fn importer_receipt_requires_complete_position_and_exact_validated_summary() {
    let assignment = assignment();
    let complete = completed(&assignment);
    assert_eq!(validate_progress(&assignment, &complete), Ok(()));
    let mut before_last = complete.clone();
    before_last.next_chunk = 0;
    assert_eq!(
        validate_progress(&assignment, &before_last),
        Err(FixtureImportError::Progress)
    );
    let mut skipped = complete.clone();
    skipped.next_chunk = 2;
    assert_eq!(
        validate_progress(&assignment, &skipped),
        Err(FixtureImportError::Progress)
    );
    let mut summary = complete.clone();
    summary.receipt.as_mut().unwrap().completion_summary = [7; 32];
    assert_eq!(
        validate_progress(&assignment, &summary),
        Err(FixtureImportError::Receipt)
    );
    let mut stale = complete;
    stale.receipt.as_mut().unwrap().binding.installation = [7; 32];
    assert_eq!(
        validate_progress(&assignment, &stale),
        Err(FixtureImportError::Receipt)
    );
}

#[test]
fn importer_progress_never_adopts_a_different_installation_or_content() {
    let assignment = assignment();
    let original = completed(&assignment);
    let mut changed = original.clone();
    changed.binding.installation = [7; 32];
    assert_eq!(
        validate_progress(&assignment, &changed),
        Err(FixtureImportError::Authority)
    );
    let mut changed = original;
    changed.binding.content_id = [7; 32];
    assert_eq!(
        validate_progress(&assignment, &changed),
        Err(FixtureImportError::Authority)
    );
}

#[test]
fn importer_leases_serialize_steps_and_old_cleanup_cannot_release_a_new_attempt() {
    let assignment = assignment();
    let first = ImportLease::acquire(&assignment).unwrap();
    assert!(matches!(
        ImportLease::acquire(&assignment),
        Err(FixtureImportError::Busy)
    ));
    abandon_expired_fetch();
    let next = ImportLease::acquire(&assignment).unwrap();
    assert_eq!(first.require_current(), Err(FixtureImportError::Authority));
    drop(first);
    assert_eq!(next.require_current(), Ok(()));
    assert!(matches!(
        ImportLease::acquire(&assignment),
        Err(FixtureImportError::Busy)
    ));
    drop(next);
    assert!(ImportLease::acquire(&assignment).is_ok());
}

struct EmptyImporter;
impl FixtureImporter for EmptyImporter {
    fn progress(
        &self,
        _: &FixtureAssignment,
    ) -> Result<Option<FixtureImportProgress>, FixtureImportError> {
        Ok(None)
    }
    fn begin(&self, _: &FixtureAssignment) -> Result<(), FixtureImportError> {
        Ok(())
    }
    fn apply_chunk(
        &self,
        _: &FixtureAssignment,
        _: u32,
        _: &[u8],
    ) -> Result<(), FixtureImportError> {
        Ok(())
    }
    fn validate_step(&self, _: &FixtureAssignment) -> Result<(), FixtureImportError> {
        Ok(())
    }
}

#[test]
fn importer_registration_has_one_owner_and_cannot_be_replaced_mid_import() {
    static IMPORTER: EmptyImporter = EmptyImporter;
    assert!(registered().is_none());
    assert_eq!(register(&IMPORTER), Ok(()));
    assert!(registered().is_some());
    assert_eq!(register(&IMPORTER), Err(FixtureImportError::Registration));
}

#[test]
fn import_retries_wait_for_infrastructure_but_stop_on_exact_invalid_content() {
    assert_eq!(permanent_failure(FixtureImportError::NotReady), None);
    assert_eq!(permanent_failure(FixtureImportError::ImporterMissing), None);
    assert_eq!(
        permanent_failure(FixtureImportError::Source(FixtureStoreError::NotReady)),
        None
    );
    assert_eq!(
        permanent_failure(FixtureImportError::Source(FixtureStoreError::Content)),
        Some(FixtureImportFailure::SourceContent)
    );
    assert_eq!(
        permanent_failure(FixtureImportError::Application { code: 13 }),
        Some(FixtureImportFailure::Application { code: 13 })
    );
}

#[test]
fn codec_failures_preserve_the_originating_code_and_stop_retrying() {
    for code in [codes::CODEC_FAILED, codes::CODEC_INVALID] {
        let error = Error::from_registered(code);
        assert_eq!(
            permanent_failure(FixtureImportError::Codec(error)),
            Some(FixtureImportFailure::Codec {
                code: error.raw_code()
            })
        );
    }
}

#[test]
fn transport_outages_and_pending_sources_remain_retryable() {
    for error in [
        FixtureImportError::Transport(Error::from_registered(codes::PLATFORM_UNAVAILABLE)),
        FixtureImportError::Source(FixtureStoreError::NotReady),
        FixtureImportError::NotReady,
    ] {
        assert_eq!(permanent_failure(error), None);
    }
}

#[test]
fn importer_replaced_installation_fences_late_cleanup() {
    let assignment = assignment();
    let lease = ImportLease::acquire(&assignment).unwrap();
    abandon_expired_fetch();
    let mut replacement = assignment;
    replacement.grant.binding.installation = [9; 32];
    let next = ImportLease::acquire(&replacement).unwrap();
    assert_eq!(lease.require_current(), Err(FixtureImportError::Authority));
    drop(lease);
    assert_eq!(next.require_current(), Ok(()));
}
