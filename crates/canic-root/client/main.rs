//! Native codec for controller operations on a sharding Root.

use canic::method::Contract;
use canic::metrics::{ChildMetrics, Request};
#[cfg(feature = "sharding")]
use canic::sharding::{Allocation, BeginShardDrain, InspectShard, ShardTarget};
#[cfg(feature = "role-issuer")]
use canic::topology::attestation::CloseRoleIssuance;
use clap::{Parser, Subcommand};
#[cfg(feature = "sharding")]
use ic_platform_types::Principal;
use serde_json::{Value, json};
#[cfg(feature = "sharding")]
use std::fmt::Write as _;
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
    ChildMetrics {
        slot: u8,
        #[command(subcommand)]
        source: Source,
    },
    /// Retain this Root’s role-verification key for approving DB receivers.
    #[cfg(feature = "role-issuer")]
    PrepareRoleKey,
    /// End proof renewal once this exact shard generation is drained and empty.
    #[cfg(feature = "role-issuer")]
    CloseRoleIssuance {
        shard: Principal,
        #[arg(value_parser = generation)]
        generation: [u8; 32],
    },
    /// Advance an exact shard retirement using its approved Store release and balance floor (JSON).
    #[cfg(feature = "role-issuer")]
    RetireShard { approval: String },
    /// Stop new placements on the exact allocation returned by inspect-shard.
    #[cfg(feature = "sharding")]
    DrainShard {
        shard: Principal,
        slot: u32,
        #[arg(value_parser = generation)]
        generation: [u8; 32],
    },
    /// Approve the subnet ceiling from the reviewed fleet plan.
    #[cfg(feature = "sharding")]
    ConfigureCapacity { approval: String },
    /// Approve one singleton role for this registry.
    #[cfg(feature = "sharding")]
    ConfigureSingleton { approval: String },
    /// Approve recurring top-ups or inspect the original approval.
    #[cfg(feature = "sharding")]
    ConfigureChildFunding { approval: String },
    /// Stop future deposits to the named child.
    #[cfg(feature = "sharding")]
    StopChildFunding {
        child: Principal,
        #[arg(long)]
        parent: Option<Principal>,
    },
    /// Approve the reserve policy from the reviewed fleet plan.
    #[cfg(feature = "sharding")]
    ConfigureStandby { approval: String },
    /// Hand a claimable spare to the exact destination Root.
    #[cfg(feature = "sharding")]
    ExportStandby {
        principal: Principal,
        destination: Principal,
        expires_at_ns: u64,
    },
    /// Observe an exact external handoff and verify its proof quarantine.
    #[cfg(feature = "sharding")]
    ImportStandby { principal: Principal },
    /// Observe the standby inventory and the exact worker allocation holding each spare.
    #[cfg(feature = "sharding")]
    InspectStandby,
    /// Confirm the hub release of this exact worker target (JSON).
    #[cfg(feature = "sharding")]
    ConfirmWorkerRelease { target: String },
    /// Stop and uninstall an exact released worker. Resume with the original deadline.
    #[cfg(feature = "sharding")]
    ResetWorker { target: String, expires_at_ns: u64 },
    /// Authorize one new empty canister. Repeat the exact request to inspect its result.
    #[cfg(feature = "sharding")]
    RefillStandby {
        root: Principal,
        sequence: u8,
        cycles: u64,
        expires_at_ns: u64,
    },
    /// Report the allocation, drain state and live placement count.
    #[cfg(feature = "sharding")]
    InspectShard { shard: Principal },
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

#[cfg(feature = "sharding")]
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
        #[cfg(feature = "role-issuer")]
        Action::Encode {
            command: Command::PrepareRoleKey,
        } => request::<canic::topology::attestation::PrepareRoleKey>(&())?,
        #[cfg(feature = "role-issuer")]
        Action::Encode {
            command: Command::CloseRoleIssuance { shard, generation },
        } => request::<CloseRoleIssuance>(&(
            shard,
            ic_wire::Bytes::new(generation.to_vec()).expect("32-byte generation"),
        ))?,
        #[cfg(feature = "role-issuer")]
        Action::Encode {
            command: Command::RetireShard { approval },
        } => request::<canic_management_protocol::stored::RetireShard>(&serde_json::from_str(
            &approval,
        )?)?,
        Action::Encode {
            command: Command::ChildMetrics { slot, source },
        } => request::<ChildMetrics>(&(
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
        #[cfg(feature = "sharding")]
        Action::Encode {
            command: Command::InspectShard { shard },
        } => request::<InspectShard>(&shard)?,
        #[cfg(feature = "sharding")]
        Action::Encode {
            command:
                Command::DrainShard {
                    shard,
                    slot,
                    generation,
                },
        } => request::<BeginShardDrain>(&ShardTarget {
            shard,
            allocation: Allocation { slot, generation },
        })?,
        #[cfg(feature = "sharding")]
        Action::Encode {
            command: Command::ConfirmWorkerRelease { target },
        } => request::<canic::sharding::standby::ConfirmRelease>(&serde_json::from_str(&target)?)?,
        #[cfg(feature = "sharding")]
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
        #[cfg(feature = "sharding")]
        Action::Encode {
            command: Command::ConfigureCapacity { approval },
        } => request::<canic_management_protocol::stored::ConfigureCapacity>(
            &serde_json::from_str(&approval)?,
        )?,
        #[cfg(feature = "sharding")]
        Action::Encode {
            command: Command::ConfigureSingleton { approval },
        } => request::<canic_management_protocol::stored::ConfigureSingleton>(
            &serde_json::from_str(&approval)?,
        )?,
        #[cfg(feature = "sharding")]
        Action::Encode {
            command: Command::ConfigureChildFunding { approval },
        } => request::<canic_management_protocol::stored::ConfigureChildFunding>(
            &serde_json::from_str(&approval)?,
        )?,
        #[cfg(feature = "sharding")]
        Action::Encode {
            command:
                Command::StopChildFunding {
                    child,
                    parent: None,
                },
        } => request::<canic_management_protocol::stored::StopChildFunding>(&child)?,
        #[cfg(feature = "sharding")]
        Action::Encode {
            command:
                Command::StopChildFunding {
                    child,
                    parent: Some(parent),
                },
        } => request::<canic_management_protocol::stored::StopApplicationChildFunding>(&(
            parent, child,
        ))?,
        #[cfg(feature = "sharding")]
        Action::Encode {
            command: Command::ConfigureStandby { approval },
        } => request::<canic_management_protocol::stored::ConfigureStandby>(
            &serde_json::from_str(&approval)?,
        )?,
        #[cfg(feature = "sharding")]
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
        #[cfg(feature = "sharding")]
        Action::Encode {
            command: Command::ImportStandby { principal },
        } => request::<canic::sharding::standby::ImportStandby>(&principal)?,
        #[cfg(feature = "sharding")]
        Action::Encode {
            command: Command::InspectStandby,
        } => request::<canic::sharding::standby::InspectStandby>(&())?,
        #[cfg(feature = "sharding")]
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
    let limit = ChildMetrics::REPLY
        .bytes
        .max(canic::logging::Logs::REPLY.bytes);
    #[cfg(feature = "sharding")]
    let limit = canic::sharding::standby::InspectStandby::REPLY
        .bytes
        .max(canic::sharding::LIMITS.bytes)
        .max(limit);
    limit
}

fn decode(bytes: &[u8]) -> Result<Value, Box<dyn std::error::Error>> {
    if let Ok(page) = canic::logging::Logs::decode_reply(bytes) {
        return Ok(serde_json::to_value(page)?);
    }
    if let Ok(page) = ChildMetrics::decode_reply(bytes) {
        return Ok(serde_json::to_value(page)?);
    }
    #[cfg(feature = "role-issuer")]
    if let Ok(reply) = canic::topology::attestation::PrepareRoleKey::decode_reply(bytes) {
        return Ok(json!({"issuer_key": reply?.into_vec()}));
    }
    #[cfg(feature = "role-issuer")]
    if let Ok(reply) = CloseRoleIssuance::decode_reply(bytes) {
        return Ok(json!({"proof_deadline_ns": reply?.to_string()}));
    }
    #[cfg(feature = "sharding")]
    if let Ok(reply) = InspectShard::decode_reply(bytes) {
        let status = reply?;
        let mut generation = String::with_capacity(64);
        for b in status.target.allocation.generation {
            write!(generation, "{b:02x}")?;
        }

        return Ok(json!({
            "shard": status.target.shard.to_text(),
            "slot": status.target.allocation.slot,
            "generation": generation,
            "draining": status.draining,
            "live_placements": status.live_placements,
        }));
    }
    #[cfg(feature = "sharding")]
    if let Ok(reply) = BeginShardDrain::decode_reply(bytes) {
        return Ok(json!({"draining": true, "live_placements": reply?}));
    }
    #[cfg(feature = "sharding")]
    if let Ok(reply) = canic_management_protocol::stored::ConfigureCapacity::decode_reply(bytes) {
        reply.map_err(|e| format!("capacity: {e:?}"))?;
        return Ok(json!({"approved":true}));
    }
    #[cfg(feature = "sharding")]
    if let Ok(reply) = canic_management_protocol::stored::ConfigureSingleton::decode_reply(bytes) {
        reply.map_err(|e| format!("singleton: {e:?}"))?;
        return Ok(json!({"approved":true}));
    }
    #[cfg(feature = "sharding")]
    if let Ok(reply) = canic_management_protocol::stored::ConfigureChildFunding::decode_reply(bytes)
    {
        return Ok(serde_json::to_value(reply?)?);
    }
    #[cfg(feature = "sharding")]
    if let Ok(reply) = canic_management_protocol::stored::StopChildFunding::decode_reply(bytes) {
        return Ok(serde_json::to_value(reply?)?);
    }
    #[cfg(feature = "sharding")]
    if let Ok(reply) =
        canic_management_protocol::stored::StopApplicationChildFunding::decode_reply(bytes)
    {
        return Ok(serde_json::to_value(reply?)?);
    }
    #[cfg(feature = "sharding")]
    if let Ok(reply) = canic_management_protocol::stored::ConfigureStandby::decode_reply(bytes) {
        reply?;
        return Ok(json!({"approved":true}));
    }
    #[cfg(feature = "sharding")]
    if let Ok(reply) = canic::sharding::standby::ExportStandby::decode_reply(bytes) {
        return Ok(serde_json::to_value(reply?)?);
    }
    #[cfg(feature = "sharding")]
    if let Ok(reply) = canic::sharding::standby::ImportStandby::decode_reply(bytes) {
        return Ok(serde_json::to_value(reply?)?);
    }
    #[cfg(feature = "sharding")]
    if let Ok(reply) = canic::sharding::standby::InspectStandby::decode_reply(bytes) {
        return Ok(serde_json::to_value(reply)?);
    }
    #[cfg(feature = "sharding")]
    if let Ok(reply) = canic::sharding::standby::RefillStandby::decode_reply(bytes) {
        return Ok(serde_json::to_value(reply?)?);
    }
    #[cfg(feature = "sharding")]
    if let Ok(reply) = canic::sharding::standby::ConfirmRelease::decode_reply(bytes) {
        reply?;
        return Ok(json!({"released": true}));
    }
    #[cfg(feature = "sharding")]
    if let Ok(reply) = canic::sharding::standby::ResetWorker::decode_reply(bytes) {
        return Ok(json!({"reset": reply?}));
    }
    #[cfg(feature = "role-issuer")]
    if let Ok(reply) = canic_management_protocol::stored::RetireShard::decode_reply(bytes) {
        return Ok(json!({"retirement":reply?}));
    }
    Err("unknown Root reply".into())
}
