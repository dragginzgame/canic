use crate::{
    Error,
    cdk::{
        api::canister_self,
        types::{Account, Nat, Principal, Subaccount},
        utils::time::now_nanos,
    },
    env::nns::{CYCLES_MINTING_CANISTER, ICP_LEDGER_CANISTER},
    interface::ic::{call::Call, derive_subaccount},
    spec::{
        ic::cycles::{IcpXdrConversionRateResponse, NotifyTopUpArgs},
        icrc::icrc1::Icrc1TransferArgs,
    },
};

//
// ===========================================================================
//  PUBLIC API
// ===========================================================================
//

/// Fetch the ICP↔︎XDR conversion rate from the Cycles Minting Canister.
///
/// Returns **ICP per XDR** (not XDR per ICP).
pub async fn get_icp_xdr_conversion_rate() -> Result<f64, Error> {
    let res = Call::unbounded_wait(*CYCLES_MINTING_CANISTER, "get_icp_xdr_conversion_rate")
        .await
        .map_err(|e| Error::custom(format!("CMC.get_icp_xdr_conversion_rate failed: {e:?}")))?;

    let rate_info: IcpXdrConversionRateResponse = res
        .candid()
        .map_err(|e| Error::custom(format!("decode error: {e:?}")))?;

    // Convert permyriad XDR/ICP → ICP/XDR
    #[allow(clippy::cast_precision_loss)]
    let xdr_per_icp = rate_info.data.xdr_permyriad_per_icp as f64 / 10_000.0;

    if xdr_per_icp <= 0.0 {
        return Err(Error::custom(format!(
            "invalid rate from CMC: xdr_per_icp={xdr_per_icp}"
        )));
    }

    Ok(1.0 / xdr_per_icp)
}

/// Convert a required number of cycles into ICP, transfer ICP to the CMC,
/// then notify CMC to mint cycles and deliver them to the caller canister.
pub async fn convert_icp_to_cycles(
    icp_ledger_account: Account,
    cycles_needed: u64,
) -> Result<(), Error> {
    //
    // Step 1: Fetch conversion rate and compute ICP needed (with buffer)
    //
    let icp_per_xdr = get_icp_xdr_conversion_rate().await?;
    let icp_needed_e8s = calculate_icp_for_cycles(cycles_needed, icp_per_xdr);

    //
    // Step 2: Generate dedicated subaccount for this conversion
    //
    let caller = canister_self();
    let subaccount = derive_subaccount(&caller, "convert_cycles");

    //
    // Step 3: Transfer ICP to CMC
    //
    let block_index = transfer_icp_to_cmc(icp_ledger_account, subaccount, icp_needed_e8s).await?;

    //
    // Step 4: Notify CMC to mint cycles and send to this canister
    //
    notify_cycles_minting_canister(block_index, caller).await?;

    Ok(())
}

//
// ===========================================================================
//  INTERNAL HELPERS
// ===========================================================================
//

/// Calculates the ICP (in e8s) required to mint the given number of cycles.
///
/// Rate interpretation: `icp_per_xdr = ICP / XDR`.
///
/// 1 XDR mints 1_000_000_000_000 (1e12) cycles.
///
/// A 5% buffer is included to accommodate temporary rate fluctuations.
#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_precision_loss)]
#[allow(clippy::cast_sign_loss)]
fn calculate_icp_for_cycles(cycles_needed: u64, icp_per_xdr: f64) -> u64 {
    const CYCLES_PER_XDR: f64 = 1_000_000_000_000.0;
    const E8S_PER_ICP: f64 = 100_000_000.0;
    const BUFFER: f64 = 1.05;

    let xdr_required = cycles_needed as f64 / CYCLES_PER_XDR;
    let icp_required = xdr_required * icp_per_xdr * BUFFER;

    (icp_required * E8S_PER_ICP).round() as u64
}

/// Perform ICRC-1 ICP transfer into the CMC’s ledger account.
async fn transfer_icp_to_cmc(
    from_account: Account,
    deposit_subaccount: Subaccount,
    icp_amount_e8s: u64,
) -> Result<u64, Error> {
    let args = Icrc1TransferArgs {
        from_subaccount: from_account.subaccount,
        to: Account {
            owner: *CYCLES_MINTING_CANISTER,
            subaccount: Some(deposit_subaccount),
        },
        amount: Nat::from(icp_amount_e8s),
        fee: None,
        memo: Some(b"cycle_conversion".to_vec()),
        created_at_time: Some(now_nanos()),
    };

    let raw = Call::unbounded_wait(*ICP_LEDGER_CANISTER, "icrc1_transfer")
        .with_args(&(args,))
        .await
        .map_err(|e| Error::custom(format!("ICP ledger transfer failed: {e:?}")))?;

    // The ICRC-1 ledger returns Result<Nat, Nat>.
    let result: Result<Nat, Nat> = raw
        .candid()
        .map_err(|e| Error::custom(format!("Failed to decode icrc1_transfer(): {e:?}")))?;

    match result {
        Ok(block) => block
            .0
            .try_into()
            .map_err(|_| Error::custom("transfer block index does not fit into u64")),
        Err(err_code) => Err(Error::custom(format!(
            "ICP transfer rejected by ledger: {err_code}"
        ))),
    }
}

/// Notify the Cycles Minting Canister to convert previously deposited ICP.
///
/// The CMC reads the deposit subaccount from the ledger block.
async fn notify_cycles_minting_canister(
    block_index: u64,
    recipient: Principal,
) -> Result<(), Error> {
    let args = NotifyTopUpArgs {
        block_index,
        canister_id: recipient,
    };

    let raw = Call::unbounded_wait(*CYCLES_MINTING_CANISTER, "notify_top_up")
        .with_args(&(args,))
        .await
        .map_err(|e| Error::custom(format!("CMC.notify_top_up transport failure: {e:?}")))?;

    let method_result: Result<(), Error> = raw
        .candid()
        .map_err(|e| Error::custom(format!("decode failure in notify_top_up: {e:?}")))?;

    method_result.map_err(|e| Error::custom(format!("CMC rejected notify_top_up: {e:?}")))
}

//
// ===========================================================================
//  TESTS
// ===========================================================================
//

#[cfg(test)]
mod tests {
    use super::calculate_icp_for_cycles;

    #[test]
    fn calculates_icp_for_cycles_with_buffer() {
        let cycles_needed = 2_000_000_000_000u64; // 2 XDR
        let icp_per_xdr = 1.25_f64; // XDR costs 1.25 ICP
        let icp_e8s = calculate_icp_for_cycles(cycles_needed, icp_per_xdr);
        assert_eq!(icp_e8s, 262_500_000); // 2 * 1.25 * 1.05 = 2.625 ICP
    }
}
