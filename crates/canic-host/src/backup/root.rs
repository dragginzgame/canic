//! Pure translation between Backup's fixed procedure and Root's bounded Wire exchange.
use canic_management_protocol::backup as snapshot;
use ic_backup::store::{
    OperationPlan, OperationStep, RootBinding, RootCodec, RootExchange, RootOutcome, RootRequest,
    SnapshotEffect,
};

/// Codec for the generation-one Canic Root snapshot executor.
/// Supply the same codec to Backup's live connection and offline store reader.
#[derive(Debug)]
pub struct SnapshotCodec;

impl RootCodec for SnapshotCodec {
    fn protocol(&self) -> [u8; 32] {
        canic_management_protocol::digest(b"canic.snapshot.execute-lookup.v1")
    }

    fn prepare(&self, input: RootRequest<'_>, frame_limit: u32) -> Option<Box<dyn RootBinding>> {
        use OperationStep as S;
        let step = match input.step {
            S::InitialStatus | S::RestoreInitialStatus => snapshot::Step::InitialStatus,
            S::ListSnapshots | S::RestoreListSnapshots => snapshot::Step::List,
            S::TakeSnapshot => snapshot::Step::Take,
            S::Metadata | S::ReadbackMetadata => snapshot::Step::Metadata,
            S::Data(n) | S::Readback(n) => snapshot::Step::Read(n),
            S::AllocateSnapshot => snapshot::Step::Allocate,
            S::Upload(n) => snapshot::Step::Upload(n),
            S::BeforeLoadStatus => snapshot::Step::BeforeLoad,
            S::LoadSnapshot => snapshot::Step::Load,
            S::AfterLoadStatus | S::FinalStatus => snapshot::Step::FinalStatus,
        };
        let effect = match input.effect {
            SnapshotEffect::Status(a) | SnapshotEffect::List(a) => {
                snapshot::Effect::Canister(a.clone())
            }
            SnapshotEffect::Take(a) => snapshot::Effect::Take(a.clone()),
            SnapshotEffect::Metadata(a) => snapshot::Effect::Metadata(a.clone()),
            SnapshotEffect::Read(a) => snapshot::Effect::Read(a.clone()),
            SnapshotEffect::Allocate(a) => snapshot::Effect::Allocate(a.clone()),
            SnapshotEffect::Upload(a) => snapshot::Effect::Upload(a.clone()),
            SnapshotEffect::Load(a) => snapshot::Effect::Load(a.clone()),
        };
        let manager = input.plan.root_manager()?;
        let request = snapshot::Request {
            operation: snapshot::Operation {
                id: input.id,
                kind: match input.plan {
                    OperationPlan::Capture(_) => snapshot::Kind::Capture,
                    OperationPlan::Restore(_) => snapshot::Kind::Restore,
                },
                approval: *input.plan.digest().as_bytes(),
                network: *input.plan.target().network.as_bytes(),
                manager: manager.root.to_text(),
                caller: input.plan.sender().to_text(),
                target: input.plan.target().canister.to_text(),
                generation: input.generation,
            },
            step,
            effect,
            arguments_hash: *input.arguments_hash.as_bytes(),
            arguments_bytes: input.arguments_bytes,
            expires_at_ns: input.expires_at_ns,
            cycles: input.cycles,
            cycle_limit: input.cycle_limit,
            reply_bytes: input.reply_bytes,
        };
        let (binding, _) = request.prepare().ok()?;
        if input.method != binding.method {
            return None;
        }
        let limits = ic_wire::Limits {
            bytes: snapshot::LIMITS.bytes.min(frame_limit as usize),
            ..snapshot::LIMITS
        };
        let frame = |method| ic_wire::Encoding::Frame {
            method,
            direction: ic_wire::Direction::Request,
        };
        Some(Box::new(Bound {
            execute: ic_wire::encode(&request, frame(snapshot::EXECUTE), limits).ok()?,
            lookup: ic_wire::encode(&binding, frame(snapshot::LOOKUP), limits).ok()?,
            binding,
        }))
    }
}

struct Bound {
    binding: snapshot::Binding,
    execute: Vec<u8>,
    lookup: Vec<u8>,
}

impl RootBinding for Bound {
    fn execute(&self) -> &[u8] {
        &self.execute
    }
    fn lookup(&self) -> &[u8] {
        &self.lookup
    }
    fn decode(&self, exchange: RootExchange, payload: &[u8]) -> Option<RootOutcome> {
        let exchange = match exchange {
            RootExchange::Execute => snapshot::Exchange::Execute,
            RootExchange::Lookup => snapshot::Exchange::Lookup,
        };
        let record = exchange
            .decode_response(&self.binding, payload)
            .ok()?
            .ok()?;
        Some(match record.outcome {
            snapshot::Outcome::Reply(bytes) => RootOutcome::Reply(bytes),
            snapshot::Outcome::Pending => RootOutcome::Pending,
            snapshot::Outcome::Refused => RootOutcome::Refused,
            snapshot::Outcome::Rejected(code, message) => RootOutcome::Rejected(code, message),
        })
    }
}
