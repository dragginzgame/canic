use crate::interface::icrc::Icrc10Standard;
use std::{cell::RefCell, collections::HashSet};

//
// ICRC 10 REGISTRY
//

thread_local! {
    static ICRC_10_REGISTRY: RefCell<HashSet<Icrc10Standard>> = RefCell::new({
        let mut set = HashSet::new();
        set.insert(Icrc10Standard::Icrc10); // Always register ICRC-10 by default

        set
    });
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
/// Icrc10Registry
///

#[derive(Default)]
pub struct Icrc10Registry();

impl Icrc10Registry {
    pub fn register(standard: Icrc10Standard) {
        ICRC_10_REGISTRY.with_borrow_mut(|reg| {
            if !reg.insert(standard) {
                panic!("standard '{standard}' has already been registered");
            }
        })
    }

    pub fn register_many(standards: &[Icrc10Standard]) {
        for standard in standards {
            Self::register(*standard)
        }
    }

    /// Checks whether the given standard is currently registered.
    #[must_use]
    pub fn is_registered(standard: Icrc10Standard) -> bool {
        ICRC_10_REGISTRY.with_borrow(|reg| reg.contains(&standard))
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
