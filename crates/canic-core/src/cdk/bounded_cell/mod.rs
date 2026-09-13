//! Module: cdk::bounded_cell
//!
//! Responsibility: enforce declared encoding bounds for singleton stable records.
//! Does not own: record schemas, admission policy, or lifecycle orchestration.
//! Boundary: validates the complete encoding before the underlying cell writes.

use crate::cdk::structures::{Memory, Storable, cell::Cell, storable::Bound};
use std::borrow::Cow;

/// A stable cell whose writes enforce the record's declared `Storable` bound.
pub struct BoundedCell<T: Storable, M: Memory> {
    cell: Cell<BoundedValue<T>, M>,
}

impl<T: Storable, M: Memory> BoundedCell<T, M> {
    /// Open a current cell or initialize a fresh bounded record.
    #[must_use]
    pub fn init(memory: M, value: T) -> Self {
        assert!(matches!(T::BOUND, Bound::Bounded { .. }));
        Self {
            cell: Cell::init(memory, BoundedValue(value)),
        }
    }

    /// Read the cached current record without serializing or cloning it.
    #[must_use]
    pub fn get(&self) -> &T {
        &self.cell.get().0
    }

    /// Validate and persist a complete replacement, returning the prior record.
    pub fn set(&mut self, value: T) -> T {
        self.cell.set(BoundedValue(value)).0
    }
}

struct BoundedValue<T>(T);

impl<T: Storable> Storable for BoundedValue<T> {
    const BOUND: Bound = T::BOUND;

    fn to_bytes(&self) -> Cow<'_, [u8]> {
        self.0.to_bytes_checked()
    }

    fn into_bytes(self) -> Vec<u8> {
        self.0.into_bytes_checked()
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        let Bound::Bounded {
            max_size,
            is_fixed_size,
        } = T::BOUND
        else {
            unreachable!("bounded cell record must declare a bound");
        };
        if is_fixed_size {
            assert_eq!(bytes.len(), max_size as usize);
        } else {
            assert!(bytes.len() <= max_size as usize);
        }
        Self(T::from_bytes(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdk::structures::VectorMemory;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct Record(Vec<u8>);

    impl Storable for Record {
        const BOUND: Bound = Bound::Bounded {
            max_size: 4,
            is_fixed_size: false,
        };
        fn to_bytes(&self) -> Cow<'_, [u8]> {
            Cow::Borrowed(&self.0)
        }
        fn into_bytes(self) -> Vec<u8> {
            self.0
        }
        fn from_bytes(bytes: Cow<[u8]>) -> Self {
            Self(bytes.into_owned())
        }
    }

    #[test]
    fn empty_singletons_allocate_only_their_cell_header() {
        fn check<T: Storable>() {
            let memory = VectorMemory::default();
            let cell = BoundedCell::init(memory.clone(), None::<T>);
            assert!(cell.get().is_none());
            assert_eq!(memory.size(), 1);
        }
        check::<crate::storage::stable::fleet_activation::FleetActivationRecord>();
        check::<crate::storage::stable::authority_restore::AuthorityRestoreFenceRecord>();
        check::<crate::storage::stable::fleet_admission_projection::FleetAdmissionProjectionRecord>(
        );
    }

    #[test]
    fn optional_record_reopens_and_rejects_oversize_before_effects() {
        let memory = VectorMemory::default();
        let mut cell = BoundedCell::init(memory.clone(), None::<Record>);
        assert_eq!(cell.get(), &None);
        cell.set(Some(Record(vec![7; 4])));
        let before = memory.borrow().clone();
        let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            cell.set(Some(Record(vec![8; 5])));
        }));
        assert!(failed.is_err());
        assert_eq!(*memory.borrow(), before);
        assert_eq!(cell.get(), &Some(Record(vec![7; 4])));
        drop(cell);
        let mut reopened = BoundedCell::init(memory.clone(), None::<Record>);
        assert_eq!(reopened.get(), &Some(Record(vec![7; 4])));
        reopened.set(None);
        assert_eq!(BoundedCell::init(memory, None::<Record>).get(), &None);
    }
}
