//! Module: canic_cli::fleet::operator_mint
//!
//! Responsibility: present and approve one retained-operation ICP conversion.
//! Boundary: host workflow owns transfer intent, receipt authentication and credit.

use crate::fleet::{EnsureOptions, FleetCommandError, now_nanoseconds};
use std::path::Path;

use candid::Principal;
use canic_core::cdk::utils::hash::decode_hex;
use canic_host::{
    fleet_ensure::{
        IcpEnsurePlatform, LoadedDesiredFleet,
        model::{
            FundingPauseRecord,
            operator_mint::{OperatorMintAuthority, OperatorMintReviewRecord},
        },
        ops::{EnsurePaths, operator_mint::transport::OperatorMintTransport, read_journal},
        plan,
        policy::operator_mint::quote,
        view::operator_mint::OperatorMintRateQuote,
        workflow::operator_mint,
    },
    icp::IcpCli,
};

pub(super) fn run(
    root: &Path,
    loaded: &LoadedDesiredFleet,
    options: &EnsureOptions,
) -> Result<(), FleetCommandError> {
    let paths = EnsurePaths::under(root, &loaded.desired.environment, &options.fleet);
    if operator_mint::fresh_quote_available(&paths)? {
        if options.apply.is_some() || options.cancel_mint.is_some() {
            return Err(FleetCommandError::Usage(
                "a fresh funding quote cannot approve or cancel a retained conversion".into(),
            ));
        }
        let transport = OperatorMintTransport::from_icp(
            &IcpCli::new(&options.icp, Some(loaded.desired.environment.clone())).with_cwd(root),
        )?;
        let quote = operator_mint::quote_fresh(
            &paths,
            &transport,
            principal(&options.mint_icp_ledger)?,
            principal(&options.mint_cmc)?,
        )?;
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"schema_version": 1, "operator_funding_quote": quote, "payment_authorized": false})
            )?
        );
        if !options.json {
            println!(
                "Read-only estimate; no journal or payment retained. The selected Fleet plan still needs its funding review and apply; preserve any retained source operation. Actual conversion depends on the rate and deposit fee at execution."
            );
        }
        return Ok(());
    }
    let mut retained = operator_mint::status(&paths)?;
    let mut rate_quote = None;
    if let Some(digest) = &options.cancel_mint {
        let review = retained.ok_or_else(|| {
            FleetCommandError::Usage("no retained conversion review to cancel".into())
        })?;
        operator_mint::cancel_review(&paths, &review.intent.authority, digest)?;
        println!(
            "{}",
            if options.json {
                "{\"operator_mint_cancelled\":true}"
            } else {
                "Unapproved conversion review cancelled; original Fleet intent retained."
            }
        );
        return Ok(());
    }
    if retained.is_none() {
        if options.apply.is_some() {
            return Err(FleetCommandError::Usage(
                "review the operator conversion before applying its digest".into(),
            ));
        }
        let (review, quote) = prepare_review(root, loaded, options, &paths)?;
        retained = Some(review);
        rate_quote = Some(quote);
    }
    let mut review = retained.expect("existing or newly retained mint review");
    if let Some(digest) = &options.apply {
        if *digest != review.review_sha256 {
            return Err(FleetCommandError::Usage(
                "apply must match the retained operator conversion digest".into(),
            ));
        }
        if review.receipt.is_none() {
            let transport = OperatorMintTransport::from_icp(
                &IcpCli::new(&options.icp, Some(loaded.desired.environment.clone())).with_cwd(root),
            )?;
            review = operator_mint::execution::apply_blocking(
                &paths,
                &review.intent.authority,
                digest,
                &transport,
            )?;
        }
    }
    let journal = read_journal(&paths)?.expect("mint review retains its journal");
    println!(
        "{}",
        render(
            &review,
            rate_quote.as_ref(),
            &journal.plan_sha256,
            options.json
        )?
    );
    Ok(())
}

impl From<canic_host::fleet_ensure::workflow::operator_mint::execution::OperatorMintExecutionError>
    for FleetCommandError
{
    fn from(
        error: canic_host::fleet_ensure::workflow::operator_mint::execution::OperatorMintExecutionError,
    ) -> Self {
        Self::MintExecution(Box::new(error))
    }
}

impl From<canic_host::fleet_ensure::workflow::EnsureWorkflowError<std::convert::Infallible>>
    for FleetCommandError
{
    fn from(
        error: canic_host::fleet_ensure::workflow::EnsureWorkflowError<std::convert::Infallible>,
    ) -> Self {
        canic_host::fleet_ensure::workflow::operator_mint::execution::OperatorMintExecutionError::from(error).into()
    }
}

impl From<canic_host::fleet_ensure::ops::operator_mint::transport::OperatorMintTransportError>
    for FleetCommandError
{
    fn from(
        error: canic_host::fleet_ensure::ops::operator_mint::transport::OperatorMintTransportError,
    ) -> Self {
        canic_host::fleet_ensure::workflow::operator_mint::execution::OperatorMintExecutionError::from(error).into()
    }
}

impl From<canic_host::fleet_ensure::ops::EnsureStateError> for FleetCommandError {
    fn from(error: canic_host::fleet_ensure::ops::EnsureStateError) -> Self {
        canic_host::fleet_ensure::workflow::EnsureWorkflowError::<std::convert::Infallible>::from(
            error,
        )
        .into()
    }
}

fn principal(value: &str) -> Result<Principal, FleetCommandError> {
    Principal::from_text(value)
        .map_err(|_| FleetCommandError::Usage("mint canister identity is not a Principal".into()))
}

fn digest(value: &str) -> Result<[u8; 32], FleetCommandError> {
    decode_hex(value)
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| {
            FleetCommandError::Usage("retained mint authority has an invalid digest".into())
        })
}

fn render(
    review: &OperatorMintReviewRecord,
    quote: Option<&OperatorMintRateQuote>,
    plan: &str,
    json: bool,
) -> Result<String, FleetCommandError> {
    let credited = review.receipt.is_some();
    if json {
        return Ok(serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": 1, "operator_mint": review, "advisory_quote": quote,
            "credit_admitted": credited, "resume_plan_sha256": plan,
        }))?);
    }
    let mut lines = vec![
        format!("operator_mint_review_sha256: {}", review.review_sha256),
        format!("operator: {}", review.intent.authority.operator),
        format!("icp_ledger: {}", review.intent.authority.icp_ledger),
        format!("cmc: {}", review.intent.authority.cmc),
        format!("cycles_ledger: {}", review.intent.authority.cycles_ledger),
        format!("transfer_amount_e8s: {}", review.intent.amount_e8s),
        format!("transfer_fee_e8s: {}", review.intent.transfer_fee_e8s),
        format!(
            "maximum_icp_debit_e8s: {}",
            u128::from(review.intent.amount_e8s) + u128::from(review.intent.transfer_fee_e8s)
        ),
        format!("payment_approved: {}", review.transfer_argument.is_some()),
        format!("credit_admitted: {credited}"),
    ];
    if let Some(quote) = quote {
        lines.push(format!(
            "advisory_rate_xdr_permyriad_per_icp: {} (timestamp {})",
            quote.xdr_permyriad_per_icp, quote.rate_timestamp_seconds
        ));
        lines.push(format!(
            "estimated_deposit_fee_cycles: {}",
            quote.estimated_deposit_fee_cycles
        ));
    }
    if let Some(receipt) = &review.receipt {
        lines.push(format!("icp_block_index: {}", receipt.icp_block_index));
        lines.push(format!(
            "deposit_block_index: {}",
            receipt.deposit_block_index
        ));
        lines.push(format!(
            "gross_minted_cycles: {}",
            receipt.gross_minted_cycles
        ));
        lines.push(format!(
            "deposit_fee_cycles: {}",
            receipt.deposit_fee_cycles
        ));
        lines.push(format!("net_credit_cycles: {}", receipt.net_credit_cycles));
        lines.push(format!(
            "resume_fleet: repeat ensure without --operator-mint using --apply {plan}"
        ));
    } else {
        lines.push(format!("transfer_outcome: {:?}", review.transfer_outcome));
        lines.push(format!(
            "notification_outcome: {:?}",
            review
                .notification
                .as_ref()
                .and_then(|n| n.outcome.as_ref())
        ));
        lines.push(format!(
            "conversion_apply: --operator-mint --apply {}",
            review.review_sha256
        ));
        lines.push("Conversion does not resume Fleet withdrawals. Unresolved outcomes retain the original payment identity.".into());
    }
    Ok(lines.join("\n"))
}

fn prepare_review(
    root: &Path,
    loaded: &LoadedDesiredFleet,
    options: &EnsureOptions,
    paths: &EnsurePaths,
) -> Result<(OperatorMintReviewRecord, OperatorMintRateQuote), FleetCommandError> {
    let mut platform = IcpEnsurePlatform::new(loaded.desired.clone(), &options.icp, root);
    let report = plan(
        root,
        &loaded.desired,
        &loaded.sha256,
        &options.fleet,
        now_nanoseconds()?,
        &mut platform,
    )?;
    let funding = report.funding_review.ok_or_else(|| {
        FleetCommandError::Usage(
            "the retained operation has no funding shortfall to convert".into(),
        )
    })?;
    let transport = OperatorMintTransport::from_icp(
        &IcpCli::new(&options.icp, Some(loaded.desired.environment.clone())).with_cwd(root),
    )?;
    let icp_ledger = principal(&options.mint_icp_ledger)?;
    let cmc = principal(&options.mint_cmc)?;
    let cycles_ledger = principal(funding.pause.cycles_ledger())?;
    let quoted = transport.quote_blocking(icp_ledger, cmc, cycles_ledger)?;
    let required = match &funding.pause {
        FundingPauseRecord::Operator(pause) => pause.required_debit_cycles,
        pause => pause
            .shortfall_cycles()
            .checked_add(pause.ledger_fee_cycles())
            .ok_or_else(|| FleetCommandError::Usage("funding debit overflow".into()))?,
    };
    let available = transport.operator_balance(cycles_ledger)?;
    let amount = quote::amount_e8s(
        required.saturating_sub(available),
        quoted.estimated_deposit_fee_cycles,
        quoted.xdr_permyriad_per_icp,
        quoted.transfer_fee_e8s,
    )
    .ok_or_else(|| {
        FleetCommandError::Usage(
            "conversion quote has a zero rate or exceeds the supported amount range".into(),
        )
    })?;
    let authority = OperatorMintAuthority {
        operation_id: digest(&report.plan.operation_id)?,
        plan_sha256: digest(&report.plan.plan_sha256)?,
        funding_review_sha256: digest(&funding.review_sha256)?,
        network_identity_sha256: transport.network_identity(),
        operator: transport.operator()?,
        icp_ledger,
        cmc,
        cycles_ledger,
    };
    let retained = operator_mint::review_conversion(
        paths,
        authority,
        amount,
        quoted.transfer_fee_e8s,
        now_nanoseconds()?,
    )?;
    Ok((retained, quoted))
}
