use crate::{
    Error,
    cdk::{api::canister_self, call::Call},
    env::nns::{CYCLES_MINTING_CANISTER, ICP_LEDGER_CANISTER},
    interface::ic::derive_subaccount,
    spec::{
        ic::cycles::{IcpXdrConversionRateResponse, NotifyTopUpArgs},
        icrc::icrc1::Icrc1TransferArgs,
    },
    types::{Account, Nat, Principal, Subaccount},
    utils::time::now_secs,
};

///
/// get_icp_xdr_conversion_rate
/// Fetch the ICP↔︎XDR conversion rate from the Cycles Minting Canister.
///
pub async fn get_icp_xdr_conversion_rate() -> Result<f64, Error> {
    let res = Call::unbounded_wait(*CYCLES_MINTING_CANISTER, "get_icp_xdr_conversion_rate").await?;

    let rate_info: IcpXdrConversionRateResponse = res.candid()?;

    // Extract the conversion rate (ICP e8s per XDR)
    #[allow(clippy::cast_precision_loss)]
    let rate = rate_info.data.xdr_permyriad_per_icp as f64 / 10000.0;

    Ok(rate)
}

///
/// convert_icp_to_cycles
/// uses the Cycles Minting Canister
///
pub async fn convert_icp_to_cycles(
    icp_ledger_account: Account,
    cycles_needed: u64,
) -> Result<(), Error> {
    // Step 1: Calculate ICP amount needed
    // Get current ICP/XDR conversion rate from CMC
    let conversion_rate = get_icp_xdr_conversion_rate().await?;
    let icp_needed = calculate_icp_for_cycles(cycles_needed, conversion_rate);

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

/// Calculates the ICP amount (in e8s) required to obtain a given number of cycles.
///
/// - `cycles_needed`: number of cycles required.
/// - `icp_per_xdr`: current exchange rate (ICP per XDR).
///
/// 1 XDR = 1e12 cycles.
/// Adds a 5% buffer to account for rate fluctuations.
#[allow(clippy::cast_sign_loss)]
#[allow(clippy::cast_precision_loss)]
#[allow(clippy::cast_possible_truncation)]
fn calculate_icp_for_cycles(cycles_needed: u64, icp_per_xdr: f64) -> u64 {
    const CYCLES_PER_XDR: f64 = 1_000_000_000_000.0;
    const E8S_PER_ICP: f64 = 100_000_000.0;
    const BUFFER_FACTOR: f64 = 1.05; // +5%

    let xdr_needed = cycles_needed as f64 / CYCLES_PER_XDR;
    let icp_needed = xdr_needed * icp_per_xdr * BUFFER_FACTOR;
    let icp_e8s = (icp_needed * E8S_PER_ICP).round();

    icp_e8s as u64
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
