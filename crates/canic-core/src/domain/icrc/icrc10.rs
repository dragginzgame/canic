const ICRC10_NAME: &str = "ICRC-10";
const ICRC10_URL: &str = "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-10";
const ICRC21_NAME: &str = "ICRC-21";
const ICRC21_URL: &str = "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-21";

/// Project the supported ICRC standards from the maintained runtime surface.
///
/// - ICRC-10 is always supported.
/// - ICRC-21 is included only when its endpoint is enabled.
/// - This is a pure, recomputed view (no storage, no persistence).
#[must_use]
pub fn supported_standards(icrc21_enabled: bool) -> Vec<(String, String)> {
    let mut supported = Vec::with_capacity(1 + usize::from(icrc21_enabled));
    supported.push((ICRC10_NAME.to_string(), ICRC10_URL.to_string()));

    if icrc21_enabled {
        supported.push((ICRC21_NAME.to_string(), ICRC21_URL.to_string()));
    }

    supported
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_standards_report_only_implemented_contracts() {
        assert_eq!(
            supported_standards(false),
            vec![(ICRC10_NAME.to_string(), ICRC10_URL.to_string())]
        );
        assert_eq!(
            supported_standards(true),
            vec![
                (ICRC10_NAME.to_string(), ICRC10_URL.to_string()),
                (ICRC21_NAME.to_string(), ICRC21_URL.to_string()),
            ]
        );
    }
}
