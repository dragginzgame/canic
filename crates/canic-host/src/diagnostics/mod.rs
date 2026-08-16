//! Module: canic_host::diagnostics
//!
//! Responsibility: render compact Canic diagnostic identities for host/operator surfaces.
//! Does not own: runtime reason construction, producer mappings, retry policy, or public projection.
//! Boundary: this host-only catalogue contains only name, origin, summary, and universally safe guidance.

mod generated;

use canic_core::diagnostics::DiagnosticCode;

///
/// DiagnosticEntry
///
/// Host-owned presentation for one current registered diagnostic reason.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticEntry {
    pub code: DiagnosticCode,
    pub name: &'static str,
    pub origin: &'static str,
    pub summary: &'static str,
    pub guidance: Option<&'static str>,
}

impl DiagnosticEntry {
    pub(super) const fn new(
        code: u16,
        name: &'static str,
        origin: &'static str,
        summary: &'static str,
        guidance: Option<&'static str>,
    ) -> Self {
        Self {
            code: DiagnosticCode::from_raw(code),
            name,
            origin,
            summary,
            guidance,
        }
    }
}

///
/// RetiredDiagnosticEntry
///
/// Minimal host identity retained after a released reason loses its final producer.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetiredDiagnosticEntry {
    pub code: DiagnosticCode,
    pub name: &'static str,
}

impl RetiredDiagnosticEntry {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the initial unreleased register has no retired rows"
        )
    )]
    pub(super) const fn new(code: u16, name: &'static str) -> Self {
        Self {
            code: DiagnosticCode::from_raw(code),
            name,
        }
    }
}

///
/// DiagnosticLookup
///
/// Lossless lookup outcome for a current, retired, or unknown raw identity.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticLookup<'a> {
    Current(&'a DiagnosticEntry),
    Retired(&'a RetiredDiagnosticEntry),
    Unknown(DiagnosticCode),
}

///
/// DiagnosticCatalog
///
/// Static host catalogue generated from the reason ledger.
///

#[derive(Debug)]
pub struct DiagnosticCatalog {
    current: &'static [DiagnosticEntry],
    retired: &'static [RetiredDiagnosticEntry],
}

impl DiagnosticCatalog {
    const fn new(
        current: &'static [DiagnosticEntry],
        retired: &'static [RetiredDiagnosticEntry],
    ) -> Self {
        Self { current, retired }
    }

    /// Look up a raw identity without guessing metadata for unknown values.
    #[must_use]
    pub fn lookup(&self, code: DiagnosticCode) -> DiagnosticLookup<'_> {
        if let Ok(index) = self
            .current
            .binary_search_by_key(&code.raw(), |entry| entry.code.raw())
        {
            return DiagnosticLookup::Current(&self.current[index]);
        }
        if let Ok(index) = self
            .retired
            .binary_search_by_key(&code.raw(), |entry| entry.code.raw())
        {
            return DiagnosticLookup::Retired(&self.retired[index]);
        }
        DiagnosticLookup::Unknown(code)
    }

    /// Iterate over current entries in numeric order.
    #[must_use]
    pub fn current_entries(&self) -> impl ExactSizeIterator<Item = &DiagnosticEntry> {
        self.current.iter()
    }

    /// Iterate over retired entries in numeric order.
    #[must_use]
    pub fn retired_entries(&self) -> impl ExactSizeIterator<Item = &RetiredDiagnosticEntry> {
        self.retired.iter()
    }
}

static CATALOG: DiagnosticCatalog =
    DiagnosticCatalog::new(generated::CURRENT_REASONS, generated::RETIRED_REASONS);

/// Return the generated host catalogue.
#[must_use]
pub const fn diagnostic_catalog() -> &'static DiagnosticCatalog {
    &CATALOG
}

/// Look up one lossless raw diagnostic identity.
#[must_use]
pub fn lookup_diagnostic(code: DiagnosticCode) -> DiagnosticLookup<'static> {
    diagnostic_catalog().lookup(code)
}

/// Render one compact diagnostic with host-owned prose.
#[must_use]
pub fn render_diagnostic(code: DiagnosticCode) -> String {
    match lookup_diagnostic(code) {
        DiagnosticLookup::Current(entry) => {
            let mut rendered = format!("{} {}: {}", entry.code, entry.name, entry.summary);
            if let Some(guidance) = entry.guidance {
                rendered.push(' ');
                rendered.push_str(guidance);
            }
            rendered
        }
        DiagnosticLookup::Retired(entry) => {
            format!("{} (retired diagnostic: {})", entry.code, entry.name)
        }
        DiagnosticLookup::Unknown(code) => format!("{code} (unknown diagnostic)"),
    }
}

// -----------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_catalogue_is_sorted_unique_and_complete() {
        let catalog = diagnostic_catalog();
        let current_codes = catalog
            .current_entries()
            .map(|entry| entry.code.raw())
            .collect::<Vec<_>>();
        let retired_codes = catalog
            .retired_entries()
            .map(|entry| entry.code.raw())
            .collect::<Vec<_>>();
        let current_names = catalog
            .current_entries()
            .map(|entry| entry.name)
            .collect::<Vec<_>>();
        let retired_names = catalog
            .retired_entries()
            .map(|entry| entry.name)
            .collect::<Vec<_>>();
        let entry_count = current_codes.len() + retired_codes.len();
        let unique_codes = current_codes
            .iter()
            .chain(&retired_codes)
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let unique_names = current_names
            .iter()
            .chain(&retired_names)
            .copied()
            .collect::<std::collections::BTreeSet<_>>();

        assert_eq!(unique_codes.len(), entry_count);
        assert_eq!(unique_names.len(), entry_count);
        assert!(unique_codes.iter().all(|code| *code != 0));
        assert!(current_codes.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(retired_codes.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn lookup_distinguishes_current_retired_and_unknown_codes() {
        static RETIRED: &[RetiredDiagnosticEntry] =
            &[RetiredDiagnosticEntry::new(23, "FORMER_IDENTITY")];

        let current = diagnostic_catalog().lookup(DiagnosticCode::from_raw(1));
        assert!(
            matches!(current, DiagnosticLookup::Current(entry) if entry.name == "ACCESS_UNAVAILABLE")
        );

        let fixture = DiagnosticCatalog::new(&[], RETIRED);
        let retired = fixture.lookup(DiagnosticCode::from_raw(23));
        assert!(
            matches!(retired, DiagnosticLookup::Retired(entry) if entry.name == "FORMER_IDENTITY")
        );

        let unknown = fixture.lookup(DiagnosticCode::from_raw(65_000));
        assert_eq!(
            unknown,
            DiagnosticLookup::Unknown(DiagnosticCode::from_raw(65_000))
        );
    }
}
