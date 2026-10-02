//! Durable capacity handoff publication under the existing Fleet operation lock.

use crate::{
    durable_io::{read_regular_bytes, write_bytes},
    fleet_ensure::{
        model::{
            EffectState,
            capacity_import::{
                CapacityImportHandoffRequestRecord, CapacityImportJournalRecord,
                CapacityImportPlanRecord, CapacityImportReservationRecord,
                retirement::{
                    CapacityImportHandoffRetirementReason, CapacityImportHandoffRetirementRecord,
                },
            },
        },
        ops::{
            EnsurePaths, EnsureStateError,
            capacity_import::{
                journal::{
                    CapacityImportJournalError, handoff_intent, retirement, reviewed, validate,
                },
                publication::{ROOT_SUBMISSION_STEPS, SOURCE_SUBMISSION_STEPS},
            },
            lock_capacity_import_operation,
        },
    },
};
use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
};

const MAXIMUM_JOURNAL_BYTES: usize = 8 * 1024 * 1024;

/// Holds the same exclusive kernel lock as Fleet ensure for each journal transaction.
pub struct CapacityImportJournalStore {
    _lock: File,
    path: PathBuf,
}

impl CapacityImportJournalStore {
    /// Ensure another import record owner borrows this exact Fleet lock.
    pub(in crate::fleet_ensure::ops::capacity_import) fn owns_paths(
        &self,
        paths: &EnsurePaths,
    ) -> bool {
        self.path == paths.plan.with_file_name("capacity-import.json")
    }

    /// Open the current import owner. The returned guard must span read, decision and write.
    pub fn open(paths: &EnsurePaths) -> Result<Self, CapacityImportJournalError> {
        let lock = lock_capacity_import_operation(paths)?;
        Ok(Self {
            _lock: lock,
            path: paths.plan.with_file_name("capacity-import.json"),
        })
    }

    /// Read a bounded regular file and reject invalid retained authority before returning it.
    pub fn read(&self) -> Result<Option<CapacityImportJournalRecord>, CapacityImportJournalError> {
        read_at(&self.path)
    }

    /// Find immutable completion by its exact review, including after a later import starts.
    pub fn completed_review(
        &self,
        review_sha256: [u8; 32],
    ) -> Result<Option<CapacityImportJournalRecord>, CapacityImportJournalError> {
        let matches = |record: &CapacityImportJournalRecord| {
            record.operation.as_ref().is_some_and(|operation| {
                operation.review.review_sha256 == review_sha256
                    && operation.released_status_candid_hex.is_some()
            })
        };
        if let Some(current) = self.read()?.filter(matches) {
            return Ok(Some(current));
        }
        let path = self
            .path
            .with_file_name("capacity-import-history")
            .join(format!(
                "{}.json",
                canic_core::cdk::utils::hash::hex_bytes(review_sha256)
            ));
        let archived = read_at(&path)?;
        if archived.as_ref().is_some_and(|record| !matches(record)) {
            return Err(CapacityImportJournalError::Integrity);
        }
        Ok(archived)
    }

    /// Persist review before approval; never overwrite an approved operation with a new plan.
    pub fn stage(
        &self,
        plan: CapacityImportPlanRecord,
    ) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
        self.stage_review(reviewed(plan)?)
    }

    /// Stage complete operator review, including generator inputs, before approval.
    pub fn stage_review(
        &self,
        record: CapacityImportJournalRecord,
    ) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
        validate(&record)?;
        if record.approved
            || record.reservation.is_some()
            || record
                .handoffs
                .iter()
                .any(|handoff| handoff.effect.is_some())
        {
            return Err(CapacityImportJournalError::Integrity);
        }
        require_completion_fits(&record)?;
        if let Some(existing) = self.read()? {
            if existing.plan == record.plan
                && existing
                    .operation
                    .as_ref()
                    .map(|operation| &operation.review)
                    == record.operation.as_ref().map(|operation| &operation.review)
            {
                return Ok(existing);
            }
            if existing.approved {
                if !crate::fleet_ensure::ops::capacity_import::publication::completed(&existing) {
                    return Err(CapacityImportJournalError::Conflict);
                }
                let digest = existing
                    .operation
                    .as_ref()
                    .ok_or(CapacityImportJournalError::Integrity)?
                    .review
                    .review_sha256;
                let archive = self
                    .path
                    .with_file_name("capacity-import-history")
                    .join(format!(
                        "{}.json",
                        canic_core::cdk::utils::hash::hex_bytes(digest)
                    ));
                let bytes = read_regular_bytes(&self.path, MAXIMUM_JOURNAL_BYTES)?;
                match read_regular_bytes(&archive, MAXIMUM_JOURNAL_BYTES) {
                    Ok(retained) if retained != bytes => {
                        return Err(CapacityImportJournalError::Conflict);
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {
                        crate::durable_io::create_new_bytes_with_parents(&archive, &bytes)?;
                    }
                    Err(error) => return Err(error.into()),
                }
            }
        }
        self.write(&record)?;
        Ok(record)
    }

    /// Publish one monotonic transition, refusing rebased balances and lost issued intent.
    pub fn save(
        &self,
        record: &CapacityImportJournalRecord,
    ) -> Result<(), CapacityImportJournalError> {
        validate(record)?;
        let previous = self.read()?.ok_or(CapacityImportJournalError::Integrity)?;
        require_monotonic(&previous, record)?;
        if previous != *record {
            self.write(record)?;
        }
        Ok(())
    }

    fn write(
        &self,
        record: &CapacityImportJournalRecord,
    ) -> Result<(), CapacityImportJournalError> {
        let bytes = serde_json::to_vec_pretty(record)?;
        if bytes.len() > MAXIMUM_JOURNAL_BYTES {
            return Err(CapacityImportJournalError::Integrity);
        }
        write_bytes(&self.path, &bytes)?;
        Ok(())
    }
}

// Reserve space for every current-contract effect and the Root reservation
// before approval. A small initial record must not strand a paid handoff later.
pub(in crate::fleet_ensure) fn require_completion_fits(
    record: &CapacityImportJournalRecord,
) -> Result<(), CapacityImportJournalError> {
    let mut largest = record.clone();
    largest.approved = true;
    largest.reservation = Some(CapacityImportReservationRecord {
        plan_sha256: record.plan.plan_sha256,
        authority: record.plan.authority.clone(),
        sources: record
            .plan
            .sources
            .iter()
            .map(|source| source.binding.canister_id)
            .collect(),
    });
    for (index, handoff) in largest.handoffs.iter_mut().enumerate() {
        let source = &record.plan.sources[index];
        let mut effect = handoff_intent(&record.plan, index, source.observed_cycles);
        effect.state = EffectState::Applied;
        effect.post_cycles = Some(source.observed_cycles);
        effect.receipt = Some(
            source
                .binding
                .canister_version
                .checked_add(1)
                .ok_or(CapacityImportJournalError::Integrity)?
                .to_string(),
        );
        handoff.effect = Some(effect);
        handoff.request = Some(CapacityImportHandoffRequestRecord {
            request_id: [u8::MAX; 32],
            ingress_expiry: u64::MAX,
            signed_envelope_hex: "ff".repeat(
                crate::fleet_ensure::ops::capacity_import::transport::MAXIMUM_ENVELOPE_BYTES,
            ),
        });
        // Size the terminal rejection case as well as the successful path before any effects.
        handoff.retirements = vec![
            CapacityImportHandoffRetirementRecord {
                request: handoff
                    .request
                    .clone()
                    .ok_or(CapacityImportJournalError::Integrity)?,
                reason: CapacityImportHandoffRetirementReason::Rejected {
                    reject_code: u8::MAX,
                    reject_message_sha256: [u8::MAX; 32],
                },
                certificate_sha256: [u8::MAX; 32],
            };
            retirement::maximum_requests(record, handoff.canister_id)?
        ];
        handoff.before_reserved_cycles = Some(u128::MAX);
        handoff.after_reserved_cycles = Some(u128::MAX);
    }
    if let Some(operation) = &mut largest.operation {
        for step in ROOT_SUBMISSION_STEPS {
            operation.submissions.insert((*step).into(), u32::MAX);
        }
        for (index, source) in record.plan.sources.iter().enumerate() {
            for phase in SOURCE_SUBMISSION_STEPS {
                operation
                    .submissions
                    .insert(format!("{index}:{phase}"), u32::MAX);
            }
            operation
                .inspections
                .insert(source.binding.canister_id.to_text(), u32::MAX);
        }
        operation
            .inspections
            .insert(record.plan.authority.root.to_text(), u32::MAX);
        let status = "f"
            .repeat(crate::fleet_ensure::ops::capacity_import::publication::MAX_STATUS_HEX_BYTES);
        operation.settled_status_candid_hex = Some(status.clone());
        operation.released_status_candid_hex = Some(status);
    }
    if serde_json::to_vec_pretty(&largest)?.len() > MAXIMUM_JOURNAL_BYTES {
        return Err(CapacityImportJournalError::Integrity);
    }
    Ok(())
}

fn require_monotonic(
    before: &CapacityImportJournalRecord,
    after: &CapacityImportJournalRecord,
) -> Result<(), CapacityImportJournalError> {
    let plan_unchanged = before.plan == after.plan;
    let approval_retained = !before.approved || after.approved;
    let reservation_retained =
        before.reservation.is_none() || before.reservation == after.reservation;
    if !plan_unchanged
        || !approval_retained
        || !reservation_retained
        || !crate::fleet_ensure::ops::capacity_import::publication::monotonic(
            before.operation.as_ref(),
            after.operation.as_ref(),
        )
    {
        return Err(CapacityImportJournalError::Conflict);
    }
    for (old, new) in before.handoffs.iter().zip(&after.handoffs) {
        if !retirement::monotonic(old, new) {
            return Err(CapacityImportJournalError::Conflict);
        }
        if old.before_reserved_cycles.is_some()
            && old.before_reserved_cycles != new.before_reserved_cycles
        {
            return Err(CapacityImportJournalError::Conflict);
        }
        if old.after_reserved_cycles.is_some()
            && old.after_reserved_cycles != new.after_reserved_cycles
        {
            return Err(CapacityImportJournalError::Conflict);
        }
        let renewal = retirement::is_renewal(old, new);
        match (&old.effect, &new.effect) {
            (None, None) => {}
            (None, Some(effect)) if effect.state == EffectState::Intent => {}
            (Some(old), Some(new)) => {
                let mut expected = old.clone();
                match (&old.state, &new.state) {
                    (EffectState::Issued, EffectState::Intent) if renewal => {
                        expected.state = EffectState::Intent;
                    }
                    (EffectState::Intent, EffectState::Issued) => {
                        expected.state = EffectState::Issued;
                    }
                    (EffectState::Issued, EffectState::Applied) => {
                        expected.state = EffectState::Applied;
                        expected.post_cycles = new.post_cycles;
                        expected.receipt.clone_from(&new.receipt);
                    }
                    _ => {}
                }
                if expected != *new {
                    return Err(CapacityImportJournalError::Conflict);
                }
            }
            _ => return Err(CapacityImportJournalError::Conflict),
        }
    }
    Ok(())
}

/// The caller already holds the shared operation lock. Import resumes through its own owner.
pub(in crate::fleet_ensure::ops) fn require_no_approved_import(
    paths: &EnsurePaths,
) -> Result<(), EnsureStateError> {
    let path = paths.plan.with_file_name("capacity-import.json");
    let pending = crate::fleet_ensure::ops::operation_selection::capacity_import_in_progress(paths)
        .map_err(|_| EnsureStateError::CapacityImportJournal {
            path: path.clone(),
            source: Box::new(CapacityImportJournalError::Integrity),
        })?;
    if pending {
        return Err(EnsureStateError::CapacityImportInProgress { path });
    }
    Ok(())
}

fn read_at(path: &Path) -> Result<Option<CapacityImportJournalRecord>, CapacityImportJournalError> {
    let bytes = match read_regular_bytes(path, MAXIMUM_JOURNAL_BYTES) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let record = serde_json::from_slice(&bytes)?;
    validate(&record)?;
    Ok(Some(record))
}
