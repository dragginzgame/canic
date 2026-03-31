use crate::{
    ids::{WasmStoreGcMode, WasmStoreGcStatus},
    storage::stable::template::{WasmStoreGcStateRecord, WasmStoreGcStateStore},
};
use canic_core::dto::error::Error;

///
/// WasmStoreGcOps
///

pub struct WasmStoreGcOps;

impl WasmStoreGcOps {
    // Return the current local wasm-store GC state record.
    #[must_use]
    pub fn status() -> WasmStoreGcStateRecord {
        WasmStoreGcStateStore::get()
    }

    // Return the current local wasm-store GC state in a boundary-safe response shape.
    #[must_use]
    pub fn snapshot() -> WasmStoreGcStatus {
        let current = Self::status();

        WasmStoreGcStatus {
            mode: current.mode,
            changed_at: current.changed_at,
            prepared_at: current.prepared_at,
            started_at: current.started_at,
            completed_at: current.completed_at,
            runs_completed: current.runs_completed,
        }
    }

    // Mark this local wasm store as prepared for store-local GC execution.
    pub fn prepare(changed_at: u64) -> Result<(), Error> {
        Self::transition_to(WasmStoreGcMode::Prepared, changed_at)
    }

    // Mark this local wasm store as actively executing store-local GC work.
    pub fn begin(changed_at: u64) -> Result<(), Error> {
        Self::transition_to(WasmStoreGcMode::InProgress, changed_at)
    }

    // Mark this local wasm store as having completed the current local GC pass.
    pub fn complete(changed_at: u64) -> Result<(), Error> {
        Self::transition_to(WasmStoreGcMode::Complete, changed_at)
    }

    // Apply one validated local GC mode transition.
    fn transition_to(next: WasmStoreGcMode, changed_at: u64) -> Result<(), Error> {
        let current = Self::status();
        let updated = transition_record(&current, next, changed_at)?;
        WasmStoreGcStateStore::set(updated);
        Ok(())
    }
}

// Validate one local wasm-store GC mode transition without touching stable state.
fn transition_record(
    current: &WasmStoreGcStateRecord,
    next: WasmStoreGcMode,
    changed_at: u64,
) -> Result<WasmStoreGcStateRecord, Error> {
    if current.mode == next {
        return Ok(current.clone());
    }

    match (current.mode, next) {
        (WasmStoreGcMode::Normal, WasmStoreGcMode::Prepared)
        | (WasmStoreGcMode::Prepared, WasmStoreGcMode::InProgress)
        | (WasmStoreGcMode::InProgress, WasmStoreGcMode::Complete) => {
            let mut updated = current.clone();
            updated.mode = next;
            updated.changed_at = changed_at;

            match next {
                WasmStoreGcMode::Prepared => {
                    updated.prepared_at = Some(changed_at);
                    updated.started_at = None;
                    updated.completed_at = None;
                }
                WasmStoreGcMode::InProgress => {
                    updated.started_at = Some(changed_at);
                    updated.completed_at = None;
                }
                WasmStoreGcMode::Complete => {
                    updated.completed_at = Some(changed_at);
                    updated.runs_completed = updated.runs_completed.saturating_add(1);
                }
                WasmStoreGcMode::Normal => {}
            }

            Ok(updated)
        }
        _ => Err(Error::conflict(format!(
            "wasm store gc transition {:?} -> {:?} is not allowed",
            current.mode, next
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::{WasmStoreGcOps, transition_record};
    use crate::{
        ids::WasmStoreGcMode,
        storage::stable::template::{WasmStoreGcStateRecord, WasmStoreGcStateStore},
    };
    use canic_core::dto::error::ErrorCode;

    #[test]
    fn transition_record_advances_monotonically() {
        let prepared = transition_record(
            &WasmStoreGcStateRecord::default(),
            WasmStoreGcMode::Prepared,
            10,
        )
        .expect("normal -> prepared must succeed");
        assert_eq!(prepared.mode, WasmStoreGcMode::Prepared);
        assert_eq!(prepared.changed_at, 10);
        assert_eq!(prepared.prepared_at, Some(10));
        assert_eq!(prepared.started_at, None);
        assert_eq!(prepared.completed_at, None);
        assert_eq!(prepared.runs_completed, 0);

        let in_progress = transition_record(&prepared, WasmStoreGcMode::InProgress, 20)
            .expect("prepared -> in progress must succeed");
        assert_eq!(in_progress.mode, WasmStoreGcMode::InProgress);
        assert_eq!(in_progress.changed_at, 20);
        assert_eq!(in_progress.prepared_at, Some(10));
        assert_eq!(in_progress.started_at, Some(20));
        assert_eq!(in_progress.completed_at, None);
        assert_eq!(in_progress.runs_completed, 0);

        let complete = transition_record(&in_progress, WasmStoreGcMode::Complete, 30)
            .expect("in progress -> complete must succeed");
        assert_eq!(complete.mode, WasmStoreGcMode::Complete);
        assert_eq!(complete.changed_at, 30);
        assert_eq!(complete.prepared_at, Some(10));
        assert_eq!(complete.started_at, Some(20));
        assert_eq!(complete.completed_at, Some(30));
        assert_eq!(complete.runs_completed, 1);
    }

    #[test]
    fn transition_record_is_idempotent_for_same_mode() {
        let current = WasmStoreGcStateRecord {
            mode: WasmStoreGcMode::Prepared,
            changed_at: 10,
            prepared_at: Some(10),
            started_at: None,
            completed_at: None,
            runs_completed: 0,
        };

        let updated = transition_record(&current, WasmStoreGcMode::Prepared, 99)
            .expect("same-mode transition must be idempotent");

        assert_eq!(updated.mode, WasmStoreGcMode::Prepared);
        assert_eq!(updated.changed_at, 10);
    }

    #[test]
    fn transition_record_rejects_invalid_order() {
        let err = transition_record(
            &WasmStoreGcStateRecord::default(),
            WasmStoreGcMode::InProgress,
            10,
        )
        .expect_err("normal -> in progress must fail");

        assert_eq!(err.code, ErrorCode::Conflict);
        assert!(err.message.contains("not allowed"));
    }

    #[test]
    fn snapshot_reflects_persisted_state() {
        WasmStoreGcStateStore::clear_for_test();
        WasmStoreGcOps::prepare(10).expect("prepare must succeed");
        let snapshot = WasmStoreGcOps::snapshot();
        assert_eq!(snapshot.mode, WasmStoreGcMode::Prepared);
        assert_eq!(snapshot.prepared_at, Some(10));
        WasmStoreGcStateStore::clear_for_test();
    }
}
