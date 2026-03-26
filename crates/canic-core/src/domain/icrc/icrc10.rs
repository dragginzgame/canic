const ICRC10_NAME: &str = "ICRC-10";
const ICRC10_URL: &str = "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-10";
const ICRC21_NAME: &str = "ICRC-21";
const ICRC21_URL: &str = "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-21";
const ICRC103_NAME: &str = "ICRC-103";
const ICRC103_URL: &str = "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-103";

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

pub struct Icrc10Registry;

impl Icrc10Registry {
    /// Returns `(name, url)` for all supported standards from the static list.
    #[must_use]
    pub fn supported_standards(
        icrc21_enabled: bool,
        icrc103_enabled: bool,
    ) -> Vec<(String, String)> {
        let mut supported =
            Vec::with_capacity(1 + usize::from(icrc21_enabled) + usize::from(icrc103_enabled));

        supported.push((ICRC10_NAME.to_string(), ICRC10_URL.to_string()));

        if icrc21_enabled {
            supported.push((ICRC21_NAME.to_string(), ICRC21_URL.to_string()));
        }

        if icrc103_enabled {
            supported.push((ICRC103_NAME.to_string(), ICRC103_URL.to_string()));
        }

        supported
    }
}
