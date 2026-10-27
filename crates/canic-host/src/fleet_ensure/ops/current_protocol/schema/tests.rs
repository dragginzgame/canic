//! Retained method authority must reject drift before a paid update.
use super::*;
use canic_contracts::cycles::Cycles;
use serde_json::json;

fn action() -> CurrentFleetProtocolAction {
    CurrentFleetProtocolAction::MaintainPoolReadiness {
        minimum_ready: 1,
        readiness_floor: Cycles::new(100),
        maximum_updates: 1,
    }
}

fn schema() -> serde_json::Value {
    let request = protocol::pool_control::REQUEST_LIMITS;
    let response = protocol::pool_control::RESPONSE_LIMITS;
    json!({"schema_version": 1, "methods": [{
        "name": protocol::pool_control::maintain::METHOD,
        "id": protocol::pool_control::maintain::METHOD_ID.to_string(),
        "kind": "update", "mode": "update",
        "requestBytes": request.bytes, "requestAllocation": request.allocation,
        "requestNodes": request.nodes, "requestDepth": request.depth,
        "responseBytes": response.bytes, "responseAllocation": response.allocation,
        "responseNodes": response.nodes, "responseDepth": response.depth
    }]})
}

#[test]
fn exact_mutation_contract_accepts_and_rejects_authority_drift() {
    let original = schema();
    admit_mutation(&action(), &serde_json::to_vec(&original).unwrap()).unwrap();
    for (field, value) in [
        ("name", json!("other")),
        ("id", json!("1")),
        ("kind", json!("query")),
        ("mode", json!("query")),
        ("requestBytes", json!(1)),
        ("requestAllocation", json!(1)),
        ("requestNodes", json!(1)),
        ("requestDepth", json!(1)),
        ("responseBytes", json!(1)),
        ("responseAllocation", json!(1)),
        ("responseNodes", json!(1)),
        ("responseDepth", json!(1)),
    ] {
        let mut changed = original.clone();
        changed["methods"][0][field] = value;
        assert!(matches!(
            admit_mutation(&action(), &serde_json::to_vec(&changed).unwrap()),
            Err(CurrentProtocolError::WireSchema(_))
        ));
    }
    let mut duplicate = original.clone();
    duplicate["methods"]
        .as_array_mut()
        .unwrap()
        .push(original["methods"][0].clone());
    assert!(matches!(
        admit_mutation(&action(), &serde_json::to_vec(&duplicate).unwrap()),
        Err(CurrentProtocolError::WireSchema(
            crate::artifact_io::WireArtifactError::Duplicate
        ))
    ));
    assert!(matches!(
        admit_mutation(&action(), b"not a schema"),
        Err(CurrentProtocolError::WireSchema(
            crate::artifact_io::WireArtifactError::Schema
        ))
    ));
}
