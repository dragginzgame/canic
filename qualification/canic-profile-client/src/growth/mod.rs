mod records;

use super::{Action, Cli, invoke, invoke_arguments};
use canic_management_protocol::{
    Approval, CREATION_CYCLES, LIMITS, MAX_WASM, RECONCILE, Refusal, STAGE, STAGE_LIMITS, STATUS,
    Stage, Status, digest,
};
use std::{
    fs::File,
    io::Read as _,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

fn read(path: &Path, maximum: usize) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > maximum {
        return Err("growth input exceeds its byte budget".into());
    }
    Ok(bytes)
}

pub(super) fn run(cli: &Cli) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let manager = cli
        .manager
        .as_deref()
        .ok_or("growth commands require --manager")?;
    if let Action::GrowthPlan {
        wasm,
        output,
        delay_seconds,
    } = &cli.command
    {
        let wasm = read(wasm, MAX_WASM)?;
        let now = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos())?;
        let execute_at_ns = now
            .checked_add(delay_seconds * 1_000_000_000)
            .ok_or("deadline overflow")?;
        let input = Stage {
            approval: Approval {
                manager: manager.into(),
                module_hash: digest(&wasm),
                creation_cycles: CREATION_CYCLES,
                execute_at_ns,
                expires_at_ns: execute_at_ns
                    .checked_add(600_000_000_000)
                    .ok_or("deadline overflow")?,
            },
            wasm,
        };
        input
            .validate(manager, now)
            .map_err(|error| format!("growth plan refused: {error:?}"))?;
        let bytes = ic_wire::encode(
            &input,
            ic_wire::Encoding::Frame {
                method: STAGE,
                direction: ic_wire::Direction::Request,
            },
            STAGE_LIMITS,
        )?;
        let document = serde_json::json!({
            "network": cli.network,
            "root_key": cli.root_key,
            "request_hex": bytes.iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
        });
        records::write(output, &serde_json::to_vec(&document)?)?;
        return Ok(
            serde_json::json!({"approval_file": output, "approval": input.approval, "bytes": input.wasm.len(), "network": cli.network}),
        );
    }
    let (id, reply, query) = match &cli.command {
        Action::GrowthApprove { approval } => {
            let document = read(approval, STAGE_LIMITS.bytes * 2 + 4096)?;
            let binding: serde_json::Value = serde_json::from_slice(&document)?;
            if binding["network"] != cli.network
                || binding["root_key"] != serde_json::to_value(&cli.root_key)?
            {
                return Err("growth approval network/root key differs from this invocation".into());
            }
            let encoded = binding["request_hex"]
                .as_str()
                .ok_or("missing growth frame")?;
            if encoded.len() % 2 != 0 || encoded.len() > STAGE_LIMITS.bytes * 2 {
                return Err("invalid growth frame length".into());
            }
            let bytes = encoded
                .as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| Ok(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?))
                .collect::<Result<Vec<u8>, Box<dyn std::error::Error>>>()?;
            let input: Stage = ic_wire::decode(
                &bytes,
                ic_wire::Encoding::Frame {
                    method: STAGE,
                    direction: ic_wire::Direction::Request,
                },
                STAGE_LIMITS,
            )?;
            if input.approval.manager != manager
                || input.approval.creation_cycles != CREATION_CYCLES
                || digest(&input.wasm) != input.approval.module_hash
            {
                return Err("growth approval binding differs from destination or artifact".into());
            }
            // Retain immutable submitted bytes; ICP reads this private copy, not a mutable caller path.
            let root = Path::new(".canic/growth-submissions");
            let (dir, held) = records::attempt(root)?;
            let submitted = dir.join("approval.bin");
            records::write_in(&held, "approval.json".as_ref(), &document)?;
            records::write_in(&held, "approval.bin".as_ref(), &bytes)?;
            eprintln!(
                "Approving one worker: manager={manager}, cycles={}, exact frame retained at {}",
                CREATION_CYCLES,
                submitted.display()
            );
            (
                STAGE,
                invoke_arguments(cli, manager, "stage", None, Some(&submitted), false)?,
                false,
            )
        }
        Action::GrowthStatus => (
            STATUS,
            invoke(
                cli,
                manager,
                "status",
                &ic_wire::encode(
                    &(),
                    ic_wire::Encoding::Frame {
                        method: STATUS,
                        direction: ic_wire::Direction::Request,
                    },
                    LIMITS,
                )?,
                true,
            )?,
            true,
        ),
        Action::GrowthReconcile => {
            let reply = invoke(
                cli,
                manager,
                "status",
                &ic_wire::encode(
                    &(),
                    ic_wire::Encoding::Frame {
                        method: STATUS,
                        direction: ic_wire::Direction::Request,
                    },
                    LIMITS,
                )?,
                true,
            )?;
            let current: Status = ic_wire::decode(
                &reply,
                ic_wire::Encoding::Frame {
                    method: STATUS,
                    direction: ic_wire::Direction::Response,
                },
                LIMITS,
            )?;
            let approval = current
                .approval
                .ok_or("manager has no admitted work to reconcile")?;
            (
                RECONCILE,
                invoke(
                    cli,
                    manager,
                    "reconcile",
                    &ic_wire::encode(
                        &approval,
                        ic_wire::Encoding::Frame {
                            method: RECONCILE,
                            direction: ic_wire::Direction::Request,
                        },
                        LIMITS,
                    )?,
                    false,
                )?,
                false,
            )
        }
        _ => unreachable!("growth command selected"),
    };
    let status = if id == STAGE {
        ic_wire::decode::<Result<Status, Refusal>>(
            &reply,
            ic_wire::Encoding::Frame {
                method: id,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )?
        .map_err(|error| format!("growth refused: {error:?}"))?
    } else {
        ic_wire::decode::<Status>(
            &reply,
            ic_wire::Encoding::Frame {
                method: id,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )?
    };
    Ok(
        serde_json::json!({"manager": manager, "query": query, "certified": !query, "result": status}),
    )
}
