//! Volatile metrics storage shared by producers in the same canister.

#[cfg(test)]
mod tests;

use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::BTreeMap,
    rc::Rc,
};

thread_local! {
    static SOURCES: RefCell<BTreeMap<TypeId, Rc<dyn Any>>> = const { RefCell::new(BTreeMap::new()) };
}

/// A static producer identity and the type of its retained metrics.
///
/// Declare a distinct type for each producer and use it at initialization
/// and at the appropriate read endpoint. Source identity comes from code.
pub trait Source: 'static {
    /// Inert storage used by this producer.
    type Storage: Default + 'static;
}

/// Read an existing source without creating it.
#[must_use]
pub fn get<S: Source>() -> Option<Rc<S::Storage>> {
    SOURCES.with_borrow(|sources| {
        sources
            .get(&TypeId::of::<S>())
            .cloned()
            .and_then(|storage| storage.downcast().ok())
    })
}

/// Get or initialize one source for this canister incarnation.
///
/// # Panics
/// Traps if the metrics registry is internally inconsistent.
#[must_use]
pub fn open<S: Source>() -> Rc<S::Storage> {
    if let Some(storage) = get::<S>() {
        return storage;
    }
    let storage = Rc::new(S::Storage::default());
    SOURCES.with_borrow_mut(|sources| {
        sources
            .entry(TypeId::of::<S>())
            .or_insert_with(|| storage.clone());
    });

    get::<S>().expect("source initialized with its declared storage type")
}
