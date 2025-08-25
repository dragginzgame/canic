use crate::{
    Error,
    cdk::call::Call,
    env::nns::{CYCLES_MINTING_CANISTER, ICP_LEDGER_CANISTER},
    interface::{ic::derive_subaccount, prelude::*},
    spec::{
        ic::cycles::{IcpXdrConversionRateResponse, NotifyTopUpArgs},
        icrc::icrc1::Icrc1TransferArgs,
    },
};

/// get_icp_xdr_conversion_rate
/// retrieved from the Cycles Minting Canister
pub async fn get_icp_xdr_conversion_rate() -> Result<f64, Error> {
    let res = Call::unbounded_wait(*CYCLES_MINTING_CANISTER, "get_icp_xdr_conversion_rate")
        .await
        .map_err(InterfaceError::from)?;

    let rate_info: IcpXdrConversionRateResponse = res.candid().map_err(InterfaceError::from)?;

    // Extract the conversion rate (ICP e8s per XDR)
    #[allow(clippy::cast_precision_loss)]
    let rate = rate_info.data.xdr_permyriad_per_icp as f64 / 10000.0;

    Ok(rate)
}

/*
/// convert_icp_to_cycles
/// uses the Cycles Minting Canister
async fn convert_icp_to_cycles(
    icp_ledger_account: Account,
    cycles_needed: u64,
) -> Result<(), Error> {
    // Step 1: Calculate ICP amount needed
    // Get current ICP/XDR conversion rate from CMC
    let conversion_rate = get_icp_xdr_conversion_rate().await?;
    let icp_needed = calculate_icp_for_cycles(cycles_needed, conversion_rate)?;

    // Step 2: Generate unique subaccount for this transaction
    let root_principal = canister_self();
    let subaccount = derive_subaccount(&root_principal, "convert_cycles");

    // Step 3: Transfer ICP to CMC with unique subaccount
    let transfer_block_index =
        transfer_icp_to_cmc(icp_ledger_account, subaccount, icp_needed).await?;

    // Step 4: Notify CMC to convert ICP to cycles and send to root canister
    notify_cycles_minting_canister(transfer_block_index, root_principal, subaccount).await?;

    Ok(())
}

/// Calculate ICP amount needed for desired cycles
fn calculate_icp_for_cycles(cycles_needed: u64, icp_per_xdr: f64) -> Result<u64, Error> {
    // 1 XDR = 1T cycles (1_000_000_000_000)
    let xdr_needed = cycles_needed as f64 / 1_000_000_000_000.0;
    let icp_needed = xdr_needed * icp_per_xdr;
    let icp_e8s = (icp_needed * 100_000_000.0) as u64; // Convert to e8s

    // Add 5% buffer for exchange rate fluctuations
    let icp_with_buffer = icp_e8s + (icp_e8s / 20);

    Ok(icp_with_buffer)
}

/// Transfer ICP to Cycles Minting Canister
async fn transfer_icp_to_cmc(
    from_account: Account,
    subaccount: Subaccount,
    icp_amount: u64,
) -> Result<u64, Error> {
    let transfer_args = Icrc1TransferArgs {
        from_subaccount: from_account.subaccount,
        to: Account {
            owner: *CYCLES_MINTING_CANISTER,
            subaccount: Some(subaccount),
        },
        amount: Nat::from(icp_amount),
        fee: None, // Use default fee
        memo: Some(b"cycle_conversion".to_vec()),
        created_at_time: Some(now_secs()),
    };

    let call_result = Call::unbounded_wait(*ICP_LEDGER_CANISTER, "icrc1_transfer")
        .with_args(&(transfer_args,))
        .await;

    match call_result {
        Ok(res) => {
            let transfer_result: Result<Nat, Nat> = res
                .candid()
                .map_err(|e| Error::custom(format!("Failed to decode transfer result: {e}")))?;
            match transfer_result {
                Ok(block_index) => {
                    let block_u64: u64 = block_index
                        .0
                        .try_into()
                        .map_err(|_| Error::custom("Block index too large".to_string()))?;
                    Ok(block_u64)
                }
                Err(e) => Err(Error::custom(format!("ICP transfer failed: {e}"))),
            }
        }
        Err(e) => Err(Error::custom(format!("Failed to transfer ICP: {e}"))),
    }
}

/// Notify CMC to convert ICP to cycles
async fn notify_cycles_minting_canister(
    block_index: u64,
    controller: Principal,
    _subaccount: Subaccount,
) -> Result<(), Error> {
    let notify_args = NotifyTopUpArgs {
        block_index,
        canister_id: controller, // Send cycles to root canister
    };

    let call_result = Call::unbounded_wait(*CYCLES_MINTING_CANISTER, "notify_top_up")
        .with_args(&(notify_args,))
        .await;

    match call_result {
        Ok(_) => Ok(()),
        Err(e) => Err(Error::custom(format!("Failed to notify CMC: {e}"))),
    }
}
*/
