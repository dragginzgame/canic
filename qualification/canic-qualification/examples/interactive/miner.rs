//! Drive the same approved pool and miner Wasms as the installed qualification.
use super::{Result, miner_pool_fixture, run_live_prompt};
use canic::method::Contract;
use canic_management_protocol::{self as management, stored};
use ex_miner_pool_hub::{PoolReport, Report, Tick};
use ic_testkit::{pocket_ic::CanisterId, server::Instance};
use std::{fmt::Write, path::Path, time::Duration};

async fn report(fleet: &miner_pool_fixture::Fleet<'_>) -> Result<Report> {
    let bytes = fleet
        .root
        .pic
        .query_call(
            fleet.pool,
            CanisterId::anonymous(),
            PoolReport::NAME,
            PoolReport::encode_request(&())?,
        )
        .await
        .map_err(|reject| reject.reject_message)?;
    Ok(PoolReport::decode_reply(&bytes)?)
}

fn display(report: &Report, limit: u8) -> String {
    let mut text = format!(
        "Round {}: {} of {} attempts confirmed, {} of {limit} workers allocated",
        report.round,
        report.completed,
        report.goal,
        report.workers.len()
    );
    for worker in &report.workers {
        write!(
            text,
            "\n  {}  {:8}  {} finds / {} attempts",
            worker.principal, worker.status, worker.blocks, worker.attempts
        )
        .expect("write to string");
    }
    text
}

pub(super) async fn session(owner: &mut Instance, artifacts: &Path, seconds: u64) -> Result<()> {
    let fleet = miner_pool_fixture::install(owner, artifacts, |_| {}).await;
    let limit = fleet.workers.approval.max_instances;
    for (name, id, request) in [
        (
            "configure_subnet_capacity",
            stored::CONFIGURE_SUBNET_CAPACITY,
            &fleet.capacity.request,
        ),
        (
            "approve_child_creation",
            stored::APPROVE_CHILD_CREATION,
            &fleet.workers.request,
        ),
    ] {
        let bytes = fleet
            .root
            .pic
            .update_call(fleet.root.id, fleet.root.operator, name, request.clone())
            .await
            .map_err(|reject| reject.reject_message)?;
        let approved: std::result::Result<(), management::Refusal> = ic_wire::decode(
            &bytes,
            ic_wire::Encoding::Frame {
                method: id,
                direction: ic_wire::Direction::Response,
            },
            stored::LIMITS,
        )?;
        approved.map_err(|why| format!("Root refused {name}: {why:?}"))?;
    }
    if let Some(reserve) = &fleet.standby {
        let bytes = fleet
            .root
            .pic
            .update_call(
                fleet.root.id,
                fleet.root.operator,
                "configure_standby",
                reserve.request.clone(),
            )
            .await
            .map_err(|reject| reject.reject_message)?;
        canic_host::application::codec::decode(&artifacts.join("root.client"), &bytes)?;
        println!(
            "Min available spares: {} (subnet fleet) · automatic creation allowance: {}.",
            reserve.approval.minimum_available, reserve.approval.maximum_creations
        );
    }
    let hint = format!(
        "Commands: miners 0..{limit}, report, exit. Try miners 3, miners 1, then miners 3 again."
    );
    println!(
        "One pool step per second; each miner attempt has a 25% chance to find a block. Root approved {limit} miner slots. {hint}"
    );
    let mut previous = report(&fleet).await?;
    let mut target = 0u8;
    let mut round = previous.round;
    let mut running = false;
    let mut last_status = String::new();
    run_live_prompt(
        "miner-pool",
        seconds,
        Some(Duration::from_secs(1)),
        async |input: Option<&str>| {
            if let Some(line) = input {
                if line == "report" {
                    return Ok(format!(
                        "Target: {target} active miners\n{}",
                        display(&report(&fleet).await?, limit)
                    ));
                }
                let Some(count) = line
                    .strip_prefix("miners ")
                    .and_then(|n| n.trim().parse::<u8>().ok())
                else {
                    return Ok(hint.clone());
                };
                if count > limit {
                    return Ok(format!("Root approved {limit} workers; choose 0..{limit}."));
                }
                if count != target || round == 0 {
                    round = round.checked_add(1).ok_or("round exhausted")?;
                }
                target = count;
                running = true;
                return Ok(format!(
                    "Target: {target} active miners. Type another command at any time."
                ));
            }
            if !running {
                return Ok(String::new());
            }
            // Pause on errors. An explicit resume of the same target keeps the exact round.
            running = false;
            let request = Tick::encode_request(&(round, target * 2))?;
            let reply = fleet
                .root
                .pic
                .update_call(fleet.pool, fleet.root.operator, Tick::NAME, request)
                .await
                .map_err(|reject| reject.reject_message)?;
            let result = Tick::decode_reply(&reply)?;
            fleet.root.pic.advance_time(Duration::from_secs(1)).await;
            fleet.root.pic.tick().await;
            let current = report(&fleet).await?;
            let mut events = Vec::new();
            for (index, worker) in current.workers.iter().enumerate() {
                let old = previous.workers.get(index);
                if old.is_none_or(|old| old.status != worker.status) {
                    events.push(format!("{}  {}", worker.principal, worker.status));
                }
                if worker.last_batch.is_some()
                    && old.and_then(|w| w.last_batch) != worker.last_batch
                {
                    let outcome = if worker.blocks > old.map_or(0, |w| w.blocks) {
                        "found a block"
                    } else {
                        "no block"
                    };
                    events.push(format!(
                        "{}  {outcome}  ({} lifetime blocks)",
                        worker.principal, worker.blocks
                    ));
                }
            }
            previous = current;
            match result.as_str() {
                "settled" if target == 0 => events.push("Pool idle.".into()),
                "settled" => {
                    round = round.checked_add(1).ok_or("round exhausted")?;
                    running = true;
                }
                "advanced" => running = true,
                status if status.ends_with(": Root allocation pending") => {
                    running = true;
                    if last_status != status {
                        events.push(status.into());
                    }
                }
                other => events.push(format!("Mining paused: {other}")),
            }
            last_status = result;
            Ok(events.join("\n"))
        },
    )
    .await
}
