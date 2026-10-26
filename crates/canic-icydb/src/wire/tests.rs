use crate::wire::{Endpoint, ID_BASE};
use ic_wire::{Error, Limits};

fn limits() -> Limits {
    Limits {
        bytes: 4096,
        allocation: 1024 * 1024,
        nodes: 1024,
        depth: 32,
    }
}

#[test]
fn public_method_identities_match_the_current_contract() {
    let methods = [
        ("icydb_query", 1),
        ("icydb_ddl", 2),
        ("icydb_update", 3),
        ("icydb_integrity", 4),
        ("icydb_fixtures_reset", 5),
        ("icydb_fixtures_load", 6),
        ("icydb_metrics", 7),
        ("icydb_metrics_reset", 8),
        ("icydb_snapshot", 9),
        ("icydb_schema", 10),
        ("icydb_schema_migrate", 11),
        ("icydb_schema_migration", 12),
    ];
    for (name, offset) in methods {
        let endpoint = Endpoint::from_name(name).unwrap();
        assert_eq!(endpoint.name(), name);
        assert_eq!(endpoint.id(), ID_BASE + offset);
        assert_eq!(Endpoint::from_id(ID_BASE + offset), Some(endpoint));
    }
    assert_eq!(Endpoint::from_name("other"), None);
    assert_eq!(Endpoint::from_id(ID_BASE), None);
    assert_eq!(Endpoint::from_id(ID_BASE + 13), None);
}

#[test]
fn envelopes_enforce_method_direction_payload_and_carried_limits() {
    let request = Endpoint::Query.encode_request("text", limits()).unwrap();
    assert_eq!(&request[..2], b"IW");
    assert_eq!(&request[2..10], &(ID_BASE + 1).to_le_bytes());
    assert_eq!(request[10], 0);
    assert_eq!(
        Endpoint::Query
            .decode_request::<String>(&request, limits())
            .unwrap(),
        "text"
    );
    assert_eq!(
        Endpoint::Metrics.decode_request::<String>(&request, limits()),
        Err(Error::Identity)
    );
    assert_eq!(
        Endpoint::Query.decode_response::<String>(&request, limits()),
        Err(Error::Identity)
    );
    let mut small = limits();
    small.bytes = request.len() - 1;
    assert!(
        Endpoint::Query
            .decode_request::<String>(&request, small)
            .is_err()
    );
    assert!(Endpoint::Query.encode_request("text", small).is_err());
    let reply = Endpoint::Query
        .encode_response(Result::<u64, u8>::Err(7), limits())
        .unwrap();
    assert_eq!(reply[10], 1);
    assert_eq!(
        Endpoint::Query.decode_response::<Result<u64, u8>>(&reply, limits()),
        Ok(Err(7))
    );
    for prefix in 0..request.len() {
        assert!(
            Endpoint::Query
                .decode_request::<String>(&request[..prefix], limits())
                .is_err()
        );
    }
    let mut trailing = request;
    trailing.push(0);
    assert!(
        Endpoint::Query
            .decode_request::<String>(&trailing, limits())
            .is_err()
    );
}

#[test]
fn lifecycle_phases_and_deserialize_only_inputs_keep_their_contract() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct ReceiveOnly(u64);
    let init = crate::wire::encode_init_arguments(42_u64, limits()).unwrap();
    assert_eq!(
        crate::wire::decode_init_arguments::<ReceiveOnly>(&init, limits()),
        Ok(ReceiveOnly(42))
    );
    assert_eq!(
        crate::wire::decode_post_upgrade_arguments::<ReceiveOnly>(&init, limits()),
        Err(Error::Identity)
    );
    let upgrade = crate::wire::encode_post_upgrade_arguments(42_u64, limits()).unwrap();
    assert_eq!(
        crate::wire::decode_post_upgrade_arguments::<ReceiveOnly>(&upgrade, limits()),
        Ok(ReceiveOnly(42))
    );
    assert_eq!(&init[2..10], &(ID_BASE + 13).to_le_bytes());
    assert_eq!(&upgrade[2..10], &(ID_BASE + 14).to_le_bytes());
}
