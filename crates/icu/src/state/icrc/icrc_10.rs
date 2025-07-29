use crate::interface::icrc::Icrc10Standard;
use std::{cell::RefCell, collections::HashSet};
use thiserror::Error as ThisError;

//
// ICRC 10 REGISTRY
//

thread_local! {
    static ICRC_10_REGISTRY: RefCell<HashSet<Icrc10Standard>> = RefCell::new(HashSet::new());
}

pub const ICRC_10_SUPPORTED_STANDARDS: &[(Icrc10Standard, &str, &str)] = &[
    (
        Icrc10Standard::Icrc10,
        "ICRC-10",
        "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-10/ICRC-10.md",
    ),
    (
        Icrc10Standard::Icrc21,
        "ICRC-21",
        "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-21/ICRC-21.md",
    ),
];

///
/// Icrc10RegistryError
///

#[derive(Debug, ThisError)]
pub enum Icrc10RegistryError {
    #[error("standard '{0}' has already been registered")]
    AlreadyRegistered(Icrc10Standard),
}

///
/// Icrc10Registry
///

#[derive(Default)]
pub struct Icrc10Registry();

impl Icrc10Registry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(standard: Icrc10Standard) -> Result<(), Icrc10RegistryError> {
        ICRC_10_REGISTRY.with_borrow_mut(|reg| {
            if !reg.insert(standard.clone()) {
                Err(Icrc10RegistryError::AlreadyRegistered(standard))
            } else {
                Ok(())
            }
        })
    }

    #[must_use]
    pub fn supported_standards() -> Vec<(String, String)> {
        ICRC_10_REGISTRY.with_borrow(|reg| {
            ICRC_10_SUPPORTED_STANDARDS
                .iter()
                .filter(|(standard, _, _)| reg.contains(standard))
                .map(|(_, name, url)| (name.to_string(), url.to_string()))
                .collect()
        })
    }
}
