//! Module: fleet_ensure::policy::operator_mint::quote
//!
//! Responsibility: checked, upward-rounded advisory conversion arithmetic.
//! Boundary: a rate sample never authorizes payment or proves actual credit.

#[cfg(test)]
mod tests;

/// Quote e8s sufficient for the shortfall plus the estimated Cycles Ledger deposit
/// fee.
///
/// One e8 at a permyriad-XDR/ICP rate yields that many cycles. The ICP transfer
/// fee is a separate debit and must fit in the same bounded e8s account equation.
#[must_use]
pub fn amount_e8s(shortfall: u128, deposit_fee: u128, rate: u64, transfer_fee: u64) -> Option<u64> {
    if shortfall == 0 || rate == 0 {
        return None;
    }
    let gross = shortfall.checked_add(deposit_fee)?;
    let rate = u128::from(rate);
    let amount = gross / rate + u128::from(!gross.is_multiple_of(rate));
    let amount = u64::try_from(amount).ok()?;
    amount.checked_add(transfer_fee)?;
    Some(amount)
}
