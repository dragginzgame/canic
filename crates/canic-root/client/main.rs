//! Native codec for controller operations on a sharding Root.

use canic::method::Contract;
use canic::metrics::performance::ChildMetrics;
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
    /// Inspect the next page of a ready child’s endpoint metrics.
    ChildMetrics { slot: u8, after: Option<String> },
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
    /// Stop new placements on the exact allocation returned by inspect-shard.
    #[cfg(feature = "sharding")]
    DrainShard {
        shard: Principal,
        slot: u32,
        #[arg(value_parser = generation)]
        generation: [u8; 32],
    },
    /// Report the allocation, drain state and live placement count.
    #[cfg(feature = "sharding")]
    InspectShard { shard: Principal },
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
        Action::Encode {
            command: Command::ChildMetrics { slot, after },
        } => request::<ChildMetrics>(&(slot, after))?,
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
    let limit = canic::sharding::LIMITS.bytes.max(limit);
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
    Err("unknown Root reply".into())
}
