//! Native codec for controller operations on Root.

use canic::method::Contract;
use canic::metrics::{ObserveChildMetrics, Request};

use canic::topology::attestation::CloseRoleIssuance;
use clap::{Parser, Subcommand};
use ic_platform_types::Principal;
use serde_json::{Value, json};

use std::io::Read as _;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    Encode {
        #[command(subcommand)]
        command: Command,
    },
    Decode,
}

#[derive(Subcommand)]
enum Command {
    /// Read Root diagnostics, newest first, optionally before a sequence in this incarnation.
    Logs { before: Option<u64> },
    /// Read one metrics source from a ready child.
    ObserveChildMetrics {
        slot: u32,
        #[command(subcommand)]
        source: Source,
    },
    /// Retain this Root’s role-verification key for approving DB receivers.
    PrepareRoleKey,
    /// End proof renewal once this exact shard generation is drained and empty.
    CloseRoleIssuance {
        shard: Principal,
        #[arg(value_parser = generation)]
        generation: [u8; 32],
    },
    /// Advance an exact shard retirement using its approved Store release and balance floor (JSON).
    RetireReleasedChild { approval: String },
    /// Apply the reviewed snapshot download list to one permanent child.
    ConfigureSnapshotReaders { approval: String },
    /// Approve the subnet ceiling from the reviewed fleet plan.
    ConfigureSubnetCapacity { approval: String },
    /// Approve a creation grant for the named requester.
    ApproveChildCreation { approval: String },
    /// Approve recurring top-ups or inspect the original approval.
    ConfigureChildFunding { approval: String },
    /// Stop future deposits to the named child.
    StopChildFunding { child: Principal },
    /// Approve the standby pool policy from the reviewed fleet plan.
    ConfigureStandby { approval: String },
    /// Hand a claimable spare to the exact destination Root.
    ExportStandby {
        principal: Principal,
        destination: Principal,
        expires_at_ns: u64,
    },
    /// Observe an exact external handoff and verify its proof quarantine.
    ImportStandby { principal: Principal },
    /// Observe the standby inventory and the exact worker allocation holding each spare.
    ObserveStandby,
    /// Confirm the hub release of this exact worker target (JSON).
    ConfirmWorkerRelease { target: String },
    /// Stop and uninstall an exact released worker. Resume with the original deadline.
    ResetWorker { target: String, expires_at_ns: u64 },
    /// Authorize one new empty canister. Repeat the exact request to inspect its result.
    RefillStandby {
        root: Principal,
        sequence: u8,
        cycles: u64,
        expires_at_ns: u64,
    },
}

#[derive(Subcommand)]
enum Source {
    Db,
    Entropy,
    Jobs,
    Timers,
    Blob,
    Endpoints { after: Option<String> },
}

fn generation(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("generation must contain 64 hex digits".into());
    }
    let mut bytes = [0; 32];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string())?;
    }

    Ok(bytes)
}

fn request<M: Contract>(input: &M::Input) -> Result<Value, String> {
    let arguments =
        M::encode_request(input).map_err(|e| format!("invalid {} request: {e:?}", M::NAME))?;

    Ok(json!({"method":M::NAME, "arguments":arguments}))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let value = match Args::parse().action {
        Action::Encode {
            command: Command::Logs { before },
        } => request::<canic::logging::Logs>(&before)?,

        Action::Encode {
            command: Command::PrepareRoleKey,
        } => request::<canic::topology::attestation::PrepareRoleKey>(&())?,

        Action::Encode {
            command: Command::CloseRoleIssuance { shard, generation },
        } => request::<CloseRoleIssuance>(&(
            shard,
            ic_wire::Bytes::new(generation.to_vec()).expect("32-byte generation"),
        ))?,

        Action::Encode {
            command: Command::RetireReleasedChild { approval },
        } => request::<canic_management_protocol::stored::RetireReleasedChild>(
            &serde_json::from_str(&approval)?,
        )?,
        Action::Encode {
            command: Command::ObserveChildMetrics { slot, source },
        } => request::<ObserveChildMetrics>(&(
            slot,
            match source {
                Source::Db => Request::Db,
                Source::Entropy => Request::Entropy,
                Source::Jobs => Request::Jobs,
                Source::Timers => Request::Timers,
                Source::Blob => Request::Blob,
                Source::Endpoints { after } => Request::Endpoints(after),
            },
        ))?,
        Action::Encode {
            command: Command::ConfirmWorkerRelease { target },
        } => request::<canic::sharding::standby::ConfirmRelease>(&serde_json::from_str(&target)?)?,
        Action::Encode {
            command:
                Command::ResetWorker {
                    target,
                    expires_at_ns,
                },
        } => request::<canic::sharding::standby::ResetWorker>(&(
            serde_json::from_str(&target)?,
            expires_at_ns,
        ))?,
        Action::Encode {
            command: Command::ConfigureSnapshotReaders { approval },
        } => request::<canic_management_protocol::stored::ConfigureSnapshotReaders>(
            &serde_json::from_str(&approval)?,
        )?,
        Action::Encode {
            command: Command::ConfigureSubnetCapacity { approval },
        } => request::<canic_management_protocol::stored::ConfigureSubnetCapacity>(
            &serde_json::from_str(&approval)?,
        )?,
        Action::Encode {
            command: Command::ApproveChildCreation { approval },
        } => request::<canic_management_protocol::stored::ApproveChildCreation>(
            &serde_json::from_str(&approval)?,
        )?,
        Action::Encode {
            command: Command::ConfigureChildFunding { approval },
        } => request::<canic_management_protocol::stored::ConfigureChildFunding>(
            &serde_json::from_str(&approval)?,
        )?,
        Action::Encode {
            command: Command::StopChildFunding { child },
        } => request::<canic_management_protocol::stored::StopChildFunding>(&child)?,

        Action::Encode {
            command: Command::ConfigureStandby { approval },
        } => request::<canic_management_protocol::stored::ConfigureStandby>(
            &serde_json::from_str(&approval)?,
        )?,

        Action::Encode {
            command:
                Command::ExportStandby {
                    principal,
                    destination,
                    expires_at_ns,
                },
        } => request::<canic::sharding::standby::ExportStandby>(&(
            principal,
            destination,
            expires_at_ns,
        ))?,

        Action::Encode {
            command: Command::ImportStandby { principal },
        } => request::<canic::sharding::standby::ImportStandby>(&principal)?,

        Action::Encode {
            command: Command::ObserveStandby,
        } => request::<canic::sharding::standby::ObserveStandby>(&())?,

        Action::Encode {
            command:
                Command::RefillStandby {
                    root,
                    sequence,
                    cycles,
                    expires_at_ns,
                },
        } => {
            request::<canic::sharding::standby::RefillStandby>(&canic::sharding::standby::Refill {
                root,
                sequence,
                cycles,
                expires_at_ns,
            })?
        }
        Action::Decode => {
            let mut bytes = Vec::new();
            std::io::stdin()
                .take((reply_limit() + 1) as u64)
                .read_to_end(&mut bytes)?;

            decode(&bytes)?
        }
    };
    println!("{value}");

    Ok(())
}

fn reply_limit() -> usize {
    canic::logging::Logs::REPLY
        .bytes
        .max(ObserveChildMetrics::REPLY.bytes)
        .max(canic::sharding::standby::ObserveStandby::REPLY.bytes)
        .max(canic::sharding::LIMITS.bytes)
}

fn decode(bytes: &[u8]) -> Result<Value, Box<dyn std::error::Error>> {
    if let Ok(page) = canic::logging::Logs::decode_reply(bytes) {
        return Ok(serde_json::to_value(page)?);
    }
    if let Ok(page) = ObserveChildMetrics::decode_reply(bytes) {
        return Ok(serde_json::to_value(page)?);
    }

    if let Ok(reply) = canic::topology::attestation::PrepareRoleKey::decode_reply(bytes) {
        return Ok(json!({"issuer_key": reply?.into_vec()}));
    }

    if let Ok(reply) = CloseRoleIssuance::decode_reply(bytes) {
        return Ok(json!({"proof_deadline_ns": reply?.to_string()}));
    }
    if let Ok(reply) =
        canic_management_protocol::stored::ConfigureSnapshotReaders::decode_reply(bytes)
    {
        reply.map_err(|e| format!("snapshot reader update: {e:?}"))?;
        return Ok(json!({"configured":true}));
    }
    if let Ok(reply) =
        canic_management_protocol::stored::ConfigureSubnetCapacity::decode_reply(bytes)
    {
        reply.map_err(|e| format!("capacity: {e:?}"))?;
        return Ok(json!({"approved":true}));
    }
    if let Ok(reply) = canic_management_protocol::stored::ApproveChildCreation::decode_reply(bytes)
    {
        reply.map_err(|e| format!("child creation: {e:?}"))?;
        return Ok(json!({"approved":true}));
    }
    if let Ok(reply) = canic_management_protocol::stored::ConfigureChildFunding::decode_reply(bytes)
    {
        return Ok(serde_json::to_value(reply?)?);
    }
    if let Ok(reply) = canic_management_protocol::stored::StopChildFunding::decode_reply(bytes) {
        return Ok(serde_json::to_value(reply?)?);
    }

    if let Ok(reply) = canic_management_protocol::stored::ConfigureStandby::decode_reply(bytes) {
        reply?;
        return Ok(json!({"approved":true}));
    }

    if let Ok(reply) = canic::sharding::standby::ExportStandby::decode_reply(bytes) {
        return Ok(serde_json::to_value(reply?)?);
    }

    if let Ok(reply) = canic::sharding::standby::ImportStandby::decode_reply(bytes) {
        return Ok(serde_json::to_value(reply?)?);
    }

    if let Ok(reply) = canic::sharding::standby::ObserveStandby::decode_reply(bytes) {
        return Ok(serde_json::to_value(reply)?);
    }

    if let Ok(reply) = canic::sharding::standby::RefillStandby::decode_reply(bytes) {
        return Ok(serde_json::to_value(reply?)?);
    }
    if let Ok(reply) = canic::sharding::standby::ConfirmRelease::decode_reply(bytes) {
        reply?;
        return Ok(json!({"released": true}));
    }
    if let Ok(reply) = canic::sharding::standby::ResetWorker::decode_reply(bytes) {
        return Ok(json!({"reset": reply?}));
    }

    if let Ok(reply) = canic_management_protocol::stored::RetireReleasedChild::decode_reply(bytes) {
        return Ok(json!({"retirement":reply?}));
    }
    Err("unknown Root reply".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_client_supports_observe_child_metrics() {
        let command = Args::try_parse_from([
            "root-client",
            "encode",
            "observe-child-metrics",
            "256",
            "timers",
        ]);
        command.unwrap();
        let reply =
            canic::metrics::ObserveChildMetrics::encode_reply(&canic::metrics::Report::Unavailable)
                .unwrap();
        decode(&reply).unwrap();
    }
}
