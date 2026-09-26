//! Exact default Ledger account observations, separate from native and reserved cycles.

use candid::Principal;
use std::collections::BTreeMap;

/// One explicitly selected default account on the source Cycles Ledger.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletedLedgerAccountView {
    pub owner: Principal,
    pub cycles: u128,
}

/// Source operator/Root accounts reconcile with original receipts; other accounts
/// remain separate observations and cannot offset historical native cycle losses.
#[derive(Debug)]
pub struct CompletedLedgerBalancesView {
    pub(in crate::fleet_ensure) ledger: Principal,
    pub(in crate::fleet_ensure) operator: CompletedLedgerAccountView,
    pub(in crate::fleet_ensure) root_accounts: BTreeMap<String, CompletedLedgerAccountView>,
    pub(in crate::fleet_ensure) other_canister_accounts:
        BTreeMap<String, CompletedLedgerAccountView>,
    pub(in crate::fleet_ensure) total_canister_ledger_cycles: u128,
}

impl CompletedLedgerBalancesView {
    /// Exact source Cycles Ledger principal on the authenticated source network.
    #[must_use]
    pub const fn ledger(&self) -> Principal {
        self.ledger
    }

    /// Operator balance after exactly the already-audited original debits.
    #[must_use]
    pub const fn operator(&self) -> &CompletedLedgerAccountView {
        &self.operator
    }

    /// Each original Root default account, without rebasing its starting balance.
    #[must_use]
    pub const fn root_accounts(&self) -> &BTreeMap<String, CompletedLedgerAccountView> {
        &self.root_accounts
    }

    /// Coordinator, Store and application/pool default accounts. These balances
    /// were outside the original Root account baseline and are never assumed zero.
    #[must_use]
    pub const fn other_canister_accounts(&self) -> &BTreeMap<String, CompletedLedgerAccountView> {
        &self.other_canister_accounts
    }

    /// Sum of all observed canister default accounts, excluding the operator.
    /// This is not a complete native/reserved/Ledger conservation assessment.
    #[must_use]
    pub const fn total_canister_ledger_cycles(&self) -> u128 {
        self.total_canister_ledger_cycles
    }
}
