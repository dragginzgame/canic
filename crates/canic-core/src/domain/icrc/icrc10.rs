use crate::config::Config;

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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Icrc10Standard {
    Icrc10,  // supported standards
    Icrc21,  // human readable representation of canister call
    Icrc103, // enhanced allowance query mechanism
}

///
/// Icrc10Registry
///
/// Runtime projection of supported ICRC standards based on static IC spec
/// and dynamic canister configuration.
///
/// - ICRC-10 is always supported.
/// - Additional standards are opt-in via config.
/// - This is a pure, recomputed view (no storage, no persistence).
///
/// Used by macro-generated endpoints in downstream crates.
///

#[allow(dead_code)]
pub struct Icrc10Registry;

#[allow(dead_code)]
impl Icrc10Registry {
    fn enabled_standards() -> Vec<Icrc10Standard> {
        let mut supported = vec![Icrc10Standard::Icrc10];

        if let Ok(config) = Config::get()
            && let Some(standards) = config.standards.as_ref()
        {
            if standards.icrc21 {
                supported.push(Icrc10Standard::Icrc21);
            }

            if standards.icrc103 {
                supported.push(Icrc10Standard::Icrc103);
            }
        }

        supported
    }

    /// Checks whether the given standard is currently registered.
    #[must_use]
    pub fn is_registered(standard: Icrc10Standard) -> bool {
        matches!(standard, Icrc10Standard::Icrc10) || Self::enabled_standards().contains(&standard)
    }

    /// Returns `(name, url)` for all supported standards from the static list.
    #[must_use]
    pub fn supported_standards() -> Vec<(String, String)> {
        let reg = Self::enabled_standards();

        ICRC_10_SUPPORTED_STANDARDS
            .iter()
            .filter(|(standard, _, _)| reg.contains(standard))
            .map(|(_, name, url)| ((*name).to_string(), (*url).to_string()))
            .collect()
    }
}
