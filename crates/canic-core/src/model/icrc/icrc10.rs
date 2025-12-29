use crate::{
    cdk::spec::icrc::icrc10::{ICRC_10_SUPPORTED_STANDARDS, Icrc10Standard},
    config::Config,
};

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

pub struct Icrc10Registry;

impl Icrc10Registry {
    fn enabled_standards() -> Vec<Icrc10Standard> {
        let config = Config::get();
        let mut supported = vec![Icrc10Standard::Icrc10];

        if let Some(standards) = config.standards.as_ref() {
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
