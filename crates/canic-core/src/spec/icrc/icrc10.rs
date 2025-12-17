use crate::spec::prelude::*;
use derive_more::Display;

///
/// ICRC 10
/// formatting instructions for each standard
///

pub const ICRC_10_SUPPORTED_STANDARDS: &[(Icrc10Standard, &str, &str)] = &[
    (
        Icrc10Standard::Icrc10,
        "ICRC-10",
        "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-10",
    ),
    (
        Icrc10Standard::Icrc21,
        "ICRC-21",
        "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-21",
    ),
    (
        Icrc10Standard::Icrc103,
        "ICRC-103",
        "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-103",
    ),
];

///
/// Icrc10Standard
/// Enumeration of well-known ICRC-10 standards with descriptive variants.
///

#[derive(Clone, Copy, Debug, Display, Eq, Hash, PartialEq)]
pub enum Icrc10Standard {
    Icrc10,  // supported standards
    Icrc21,  // human readable representation of canister call
    Icrc103, // enhanced allowance query mechanism
}

///
/// Icrc10SupportedStandard
/// Response payload describing a single supported standard entry.
///

#[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
pub struct Icrc10SupportedStandard {
    pub url: String,
    pub name: String,
}
