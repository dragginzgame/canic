use candid::{CandidType, Principal, encode_one};
use serde::{Deserialize, Serialize};

const MAX_FLEET_PRINCIPALS: usize = 256;
const MAX_RULES: usize = 32;
const MAX_RULE_PRINCIPAL_REFS: usize = 128;
const MAX_ROOTS: usize = 4_096;
const MAX_PARTICIPANTS_PER_FLEET: usize = 4_096;
const STATUS_PAGE: usize = 32;

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct FleetBinding {
    canonical_network_id: String,
    fleet_id: String,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
enum Selector {
    ComponentSpec { component_spec: String },
    ComponentInstance { component_instance: String },
    FleetSubnetRoot { placement_subnet: Principal },
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct Rule {
    selector: Selector,
    principals: Vec<Principal>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct Policy {
    schema_version: u16,
    fleet: FleetBinding,
    generation: u64,
    fleet_principals: Vec<Principal>,
    rules: Vec<Rule>,
    policy_digest: [u8; 32],
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
enum Phase {
    Planned,
    Preparing,
    Fenced,
    Activating,
    Opening,
    Converged,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct RootProgress {
    root: Principal,
    placement_subnet: Principal,
    phase: Phase,
    prepare_receipt: Option<[u8; 32]>,
    activate_receipt: Option<[u8; 32]>,
    open_receipt: Option<[u8; 32]>,
    failure_code: Option<u16>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct ParticipantProgress {
    target: Principal,
    component_instance: String,
    phase: Phase,
    projection_digest: [u8; 32],
    prepare_receipt: Option<[u8; 32]>,
    activate_receipt: Option<[u8; 32]>,
    open_receipt: Option<[u8; 32]>,
    failure_code: Option<u16>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct CoordinatorState {
    schema_version: u16,
    current: Policy,
    prepared: Option<Policy>,
    operation_id: [u8; 32],
    request_hash: [u8; 32],
    phase: Phase,
    roots: Vec<RootProgress>,
    last_result_roots: Vec<RootProgress>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct RootState {
    schema_version: u16,
    current: Policy,
    prepared: Option<Policy>,
    operation_id: [u8; 32],
    request_hash: [u8; 32],
    phase: Phase,
    participants: Vec<ParticipantProgress>,
    last_result_participants: Vec<ParticipantProgress>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct Projection {
    schema_version: u16,
    fleet: FleetBinding,
    coordinator: Principal,
    target: Principal,
    component_instance: String,
    generation: u64,
    policy_digest: [u8; 32],
    projection_digest: [u8; 32],
    principals: Vec<Principal>,
    fenced: bool,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct ParticipantState {
    schema_version: u16,
    active: Projection,
    prepared: Option<Projection>,
    operation_id: [u8; 32],
    request_hash: [u8; 32],
    last_receipt: [u8; 32],
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct PrepareRootCommand {
    operation_id: [u8; 32],
    expected_generation: u64,
    expected_policy_digest: [u8; 32],
    successor: Policy,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct PrepareParticipantCommand {
    operation_id: [u8; 32],
    expected_generation: u64,
    expected_policy_digest: [u8; 32],
    successor: Projection,
}

fn principal(index: usize) -> Principal {
    let mut bytes = [0_u8; 29];
    bytes[..8].copy_from_slice(&(index as u64).to_be_bytes());
    bytes[8..].fill(0xa5);
    Principal::from_slice(&bytes)
}

fn policy() -> Policy {
    let fleet_principals = (0..MAX_FLEET_PRINCIPALS).map(principal).collect::<Vec<_>>();
    let refs_per_rule = MAX_RULE_PRINCIPAL_REFS / MAX_RULES;
    let rules = (0..MAX_RULES)
        .map(|index| Rule {
            selector: Selector::ComponentInstance {
                component_instance: format!("{index:064x}"),
            },
            principals: (0..refs_per_rule)
                .map(|offset| {
                    fleet_principals[(index * refs_per_rule + offset) % fleet_principals.len()]
                })
                .collect(),
        })
        .collect();
    Policy {
        schema_version: 1,
        fleet: FleetBinding {
            canonical_network_id: "n".repeat(40),
            fleet_id: "f".repeat(40),
        },
        generation: u64::MAX,
        fleet_principals,
        rules,
        policy_digest: [0xff; 32],
    }
}

fn root_progress(index: usize) -> RootProgress {
    RootProgress {
        root: principal(10_000 + index),
        placement_subnet: principal(20_000 + index),
        phase: Phase::Opening,
        prepare_receipt: Some([0xaa; 32]),
        activate_receipt: Some([0xbb; 32]),
        open_receipt: Some([0xcc; 32]),
        failure_code: Some(u16::MAX),
    }
}

fn participant_progress(index: usize) -> ParticipantProgress {
    ParticipantProgress {
        target: principal(30_000 + index),
        component_instance: format!("{index:064x}"),
        phase: Phase::Opening,
        projection_digest: [0xdd; 32],
        prepare_receipt: Some([0xaa; 32]),
        activate_receipt: Some([0xbb; 32]),
        open_receipt: Some([0xcc; 32]),
        failure_code: Some(u16::MAX),
    }
}

fn projection() -> Projection {
    Projection {
        schema_version: 1,
        fleet: FleetBinding {
            canonical_network_id: "n".repeat(40),
            fleet_id: "f".repeat(40),
        },
        coordinator: principal(90_000),
        target: principal(90_001),
        component_instance: "f".repeat(64),
        generation: u64::MAX,
        policy_digest: [0xee; 32],
        projection_digest: [0xff; 32],
        principals: (0..MAX_FLEET_PRINCIPALS).map(principal).collect(),
        fenced: true,
    }
}

fn cbor_len<T: Serialize>(value: &T) -> usize {
    let mut bytes = Vec::new();
    ciborium::into_writer(value, &mut bytes).expect("CBOR");
    bytes.len()
}

fn candid_len<T: CandidType + Serialize>(value: &T) -> usize {
    encode_one(value).expect("Candid").len()
}

fn main() {
    let policy = policy();
    let projection = projection();
    let roots = (0..MAX_ROOTS).map(root_progress).collect::<Vec<_>>();
    let participants = (0..MAX_PARTICIPANTS_PER_FLEET)
        .map(participant_progress)
        .collect::<Vec<_>>();
    let coordinator = CoordinatorState {
        schema_version: 1,
        current: policy.clone(),
        prepared: Some(policy.clone()),
        operation_id: [0x11; 32],
        request_hash: [0x22; 32],
        phase: Phase::Opening,
        roots: roots.clone(),
        last_result_roots: roots,
    };
    let root = RootState {
        schema_version: 1,
        current: policy.clone(),
        prepared: Some(policy.clone()),
        operation_id: [0x11; 32],
        request_hash: [0x22; 32],
        phase: Phase::Opening,
        participants: participants.clone(),
        last_result_participants: participants,
    };
    let participant = ParticipantState {
        schema_version: 1,
        active: projection.clone(),
        prepared: Some(projection.clone()),
        operation_id: [0x11; 32],
        request_hash: [0x22; 32],
        last_receipt: [0x33; 32],
    };
    let root_command = PrepareRootCommand {
        operation_id: [0x11; 32],
        expected_generation: u64::MAX - 1,
        expected_policy_digest: [0x22; 32],
        successor: policy.clone(),
    };
    let participant_command = PrepareParticipantCommand {
        operation_id: [0x11; 32],
        expected_generation: u64::MAX - 1,
        expected_policy_digest: [0x22; 32],
        successor: projection,
    };
    let status_page = (0..STATUS_PAGE)
        .map(participant_progress)
        .collect::<Vec<_>>();

    println!("policy_candid_bytes={}", candid_len(&policy));
    println!("policy_cbor_bytes={}", cbor_len(&policy));
    println!(
        "root_prepare_command_candid_bytes={}",
        candid_len(&root_command)
    );
    println!(
        "participant_prepare_command_candid_bytes={}",
        candid_len(&participant_command)
    );
    println!(
        "participant_status_page_candid_bytes={}",
        candid_len(&status_page)
    );
    println!("coordinator_state_cbor_bytes={}", cbor_len(&coordinator));
    println!("root_state_cbor_bytes={}", cbor_len(&root));
    println!("participant_state_cbor_bytes={}", cbor_len(&participant));
}
