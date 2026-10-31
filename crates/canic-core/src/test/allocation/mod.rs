//! Measure allocations on the selected native test thread around production operations.
//! The system allocator owns memory; this observer never changes allocation behavior.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static OBSERVED: Cell<AllocationCost> = const { Cell::new(AllocationCost::EMPTY) };
}

#[global_allocator]
static ALLOCATOR: ObservedSystem = ObservedSystem;

struct ObservedSystem;

// SAFETY: every allocation and deallocation delegates unchanged to System.
unsafe impl GlobalAlloc for ObservedSystem {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller supplies System's required valid layout.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            observe(layout.size(), 0);
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller supplies System's required valid layout.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            observe(layout.size(), 0);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: the caller supplies the original live allocation and its layout.
        unsafe { System.dealloc(pointer, layout) };
        observe(0, layout.size());
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        // SAFETY: System receives the caller's live allocation, layout and valid new size.
        let resized = unsafe { System.realloc(pointer, layout, size) };
        if !resized.is_null() {
            observe(size, layout.size());
        }
        resized
    }
}

/// Requested allocation sizes, including table capacity and control bytes.
#[derive(Clone, Copy, Debug)]
pub struct AllocationCost {
    pub allocated_bytes: usize,
    pub released_bytes: usize,
    pub allocations: usize,
    pub retained_bytes: i128,
    pub peak_bytes: i128,
}

impl AllocationCost {
    const EMPTY: Self = Self {
        allocated_bytes: 0,
        released_bytes: 0,
        allocations: 0,
        retained_bytes: 0,
        peak_bytes: 0,
    };
}

fn observe(allocated: usize, released: usize) {
    // Constant TLS cells do not allocate. Teardown on other threads is ignored.
    let _ = ACTIVE.try_with(|active| {
        if active.get() {
            OBSERVED.with(|observed| {
                let mut cost = observed.get();
                cost.allocated_bytes += allocated;
                cost.released_bytes += released;
                cost.allocations += usize::from(allocated != 0);
                cost.retained_bytes += allocated as i128 - released as i128;
                cost.peak_bytes = cost.peak_bytes.max(cost.retained_bytes);
                observed.set(cost);
            });
        }
    });
}

struct Observation;

impl Drop for Observation {
    fn drop(&mut self) {
        ACTIVE.set(false);
    }
}

/// Observe one synchronous scope; another test thread cannot contaminate its totals.
pub fn measure<T>(operation: impl FnOnce() -> T) -> (T, AllocationCost) {
    assert!(!ACTIVE.get(), "allocation observations cannot nest");
    OBSERVED.set(AllocationCost::EMPTY);
    ACTIVE.set(true);
    let observation = Observation;
    let result = operation();
    drop(observation);
    (result, OBSERVED.get())
}
