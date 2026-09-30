//! Free preparation failures cannot consume any approval inspection allowance.

use super::*;
use crate::fleet_ensure::{
    model::capacity_import::CapacityImportPlanRecord,
    policy::capacity_import::tests::plan,
    view::capacity_import::{CapacityImportDestinationView, CapacityImportSourceView},
    workflow::capacity_import::tests::reviewed_journal,
};
use std::{
    future::{Future, ready},
    marker::PhantomData,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

struct Observer {
    fail_destination: bool,
    paid_calls: Arc<AtomicUsize>,
}

struct Prepared<View> {
    paid_calls: Arc<AtomicUsize>,
    view: PhantomData<View>,
}

impl<View: Send> PreparedCapacityImportObservation for Prepared<View> {
    type View = View;

    fn observe(self) -> impl Future<Output = Result<Self::View, CapacityImportJournalError>> {
        self.paid_calls.fetch_add(1, Ordering::Relaxed);
        ready(Err(CapacityImportJournalError::Unresolved))
    }
}

impl CapacityImportObserver for Observer {
    type Destination = Prepared<CapacityImportDestinationView>;
    type Source = Prepared<CapacityImportSourceView>;

    fn prepare_destination(
        &mut self,
        _: &CapacityImportPlanRecord,
    ) -> impl Future<Output = Result<Self::Destination, CapacityImportJournalError>> {
        if self.fail_destination {
            return ready(Err(CapacityImportJournalError::InfrastructureChanged));
        }
        ready(Ok(Prepared {
            paid_calls: Arc::clone(&self.paid_calls),
            view: PhantomData,
        }))
    }

    fn prepare_source(
        &mut self,
        _: &CapacityImportPlanRecord,
        _: candid::Principal,
    ) -> impl Future<Output = Result<Self::Source, CapacityImportJournalError>> {
        ready(Err(CapacityImportJournalError::InventoryInvalid))
    }
}

#[test]
fn free_approval_failures_preserve_all_inspections_across_reopen() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    for fail_destination in [true, false] {
        let (directory, paths, store) = reviewed_journal(plan());
        let original = store.read().unwrap().unwrap();
        drop(store);
        let paid_calls = Arc::new(AtomicUsize::new(0));
        let mut observer = Observer {
            fail_destination,
            paid_calls: Arc::clone(&paid_calls),
        };
        let limit = original
            .operation
            .as_ref()
            .unwrap()
            .review
            .maximum_management_observations_per_canister;
        for _ in 0..=limit {
            let store = CapacityImportJournalStore::open(&paths).unwrap();
            let mut record = store.read().unwrap().unwrap();
            let result = runtime.block_on(approve(&store, &mut record, &mut observer));
            if fail_destination {
                assert!(matches!(
                    result,
                    Err(CapacityImportJournalError::InfrastructureChanged)
                ));
            } else {
                assert!(matches!(
                    result,
                    Err(CapacityImportJournalError::InventoryInvalid)
                ));
            }
            assert_eq!(paid_calls.load(Ordering::Relaxed), 0);
            assert_eq!(record, original);
            assert_eq!(store.read().unwrap().unwrap(), original);
        }
        std::fs::remove_dir_all(directory).unwrap();
    }
}
