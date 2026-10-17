//! Module: artifact_io::wasm::install_policy
//!
//! Responsibility: apply Canic-selected replica structural limits.
//! Does not own: generic Wasm inspection, which stays in Host Artifacts.
//!
//! Thresholds retain the previously selected replica revision
//! `9499f64bda8bcf087188dd8a1bb594115640ba9e`: 12 MiB code bodies, 50,000 local
//! functions and 1,000 imported plus local globals. Passing these checks does not
//! establish instruction validity or every other replica installation constraint.

#[cfg(test)]
mod tests;

use ic_host_artifacts::wasm::WasmFacts;

///
/// LimitUsage
///
/// Canic structural observation and its inclusive selected limit.
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct LimitUsage {
    /// Observed bytes or entries.
    pub observed: u64,
    /// Inclusive maximum.
    pub limit: u64,
}

impl LimitUsage {
    /// Whether the observation strictly exceeds the selected limit.
    #[must_use]
    pub const fn exceeded(self) -> bool {
        self.observed > self.limit
    }

    /// Remaining allowance, negative when exceeded; arithmetic cannot wrap.
    #[must_use]
    pub fn headroom(self) -> i128 {
        i128::from(self.limit) - i128::from(self.observed)
    }
}

///
/// InstallReport
///
/// Canic structural headroom; this does not establish installation success.
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct InstallReport {
    /// Remaining code-body allowance.
    pub code_body_bytes: LimitUsage,
    /// Remaining defined-function allowance.
    pub defined_functions: LimitUsage,
    /// Remaining total-global allowance.
    pub globals: LimitUsage,
}

impl InstallReport {
    /// Compare already-inspected facts without reparsing or allocating.
    #[must_use]
    pub(super) fn from_facts(facts: &WasmFacts<'_>) -> Self {
        Self {
            code_body_bytes: LimitUsage {
                observed: facts.code_body_bytes as u64,
                limit: 12 * 1024 * 1024,
            },
            defined_functions: LimitUsage {
                observed: u64::from(facts.defined_functions),
                limit: 50_000,
            },
            globals: LimitUsage {
                observed: u64::from(facts.defined_globals) + u64::from(facts.imported_globals),
                limit: 1_000,
            },
        }
    }
}
