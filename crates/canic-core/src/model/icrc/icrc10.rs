use crate::{
    config::Config,
    spec::icrc::icrc10::{ICRC_10_SUPPORTED_STANDARDS, Icrc10Standard},
};

///
/// Icrc10Registry
///

#[derive(Default)]
pub struct Icrc10Registry();

impl Icrc10Registry {
    fn standards() -> Vec<Icrc10Standard> {
        let config = Config::try_get();
        let mut supported = vec![Icrc10Standard::Icrc10];

        if let Some(standards) = config.as_ref().and_then(|cfg| cfg.standards.as_ref()) {
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
