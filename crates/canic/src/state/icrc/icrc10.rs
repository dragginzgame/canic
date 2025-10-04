use crate::{config::Config, spec::icrc::icrc10::Icrc10Standard};

//
// ICRC 10
// this is now a wrapper around the Config state
//

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
/// Icrc10Registry
///

#[derive(Default)]
pub struct Icrc10Registry();

impl Icrc10Registry {
    fn standards() -> Vec<Icrc10Standard> {
        let config = Config::try_get().unwrap();

        let mut supported = vec![Icrc10Standard::Icrc10];

        #[allow(clippy::collapsible_if)]
        if let Some(standards) = &config.standards {
            if standards.icrc21 {
                supported.push(Icrc10Standard::Icrc21);
            }
            if standards.icrc103 {
                supported.push(Icrc10Standard::Icrc103);
            }

            // if standards.
        }

        supported
    }

    /// Checks whether the given standard is currently registered.
    #[must_use]
    pub fn is_registered(standard: Icrc10Standard) -> bool {
        Self::standards().contains(&standard)
    }

    /// Returns `(name, url)` for all supported standards from the static list.
    #[must_use]
    pub fn supported_standards() -> Vec<(String, String)> {
        let reg = Self::standards();

        ICRC_10_SUPPORTED_STANDARDS
            .iter()
            .filter(|(standard, _, _)| reg.contains(standard))
            .map(|(_, name, url)| ((*name).to_string(), (*url).to_string()))
            .collect()
    }
}
