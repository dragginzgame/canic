use super::*;

fn response() -> protocol::Response {
    protocol::Response {
        schema: [7; 32],
        admission: protocol::Admission::Maintenance,
        selection: protocol::Selection::Catalogue(
            ic_wire::List::new(vec![protocol::Name::new("Item".to_owned()).unwrap()]).unwrap(),
        ),
    }
}

fn frame(response: &protocol::Reply, method: u64, direction: Direction) -> Vec<u8> {
    ic_wire::encode(response, Encoding::Frame { method, direction }, RPC_LIMITS).unwrap()
}

#[test]
fn inspection_frames_and_selection_are_exact() {
    let response = response();
    let request = protocol::Request::Catalogue;
    let bytes = frame(
        &Ok(response.clone()),
        protocol::INSPECT,
        Direction::Response,
    );
    assert_eq!(decode(&bytes, &request).unwrap(), response);
    assert!(render(&response).contains("Maintenance"));
    assert!(!render(&response).contains("Readable"));
    for (method, direction) in [
        (0, Direction::Response),
        (protocol::INSPECT, Direction::Request),
    ] {
        assert!(decode(&frame(&Ok(response.clone()), method, direction), &request).is_err());
    }
    assert!(decode(&bytes[..bytes.len() - 1], &request).is_err());
    assert!(decode(&vec![0; RPC_LIMITS.bytes + 1], &request).is_err());
    assert!(
        decode(
            &frame(
                &Err(protocol::Refusal::SchemaChanged),
                protocol::INSPECT,
                Direction::Response
            ),
            &request
        )
        .unwrap_err()
        .contains("SchemaChanged")
    );
    let selected = protocol::Request::Entity {
        schema: [7; 32],
        name: protocol::Name::new("Item".to_owned()).unwrap(),
        count: true,
    };
    assert!(decode(&bytes, &selected).is_err());
}

#[test]
fn selected_identity_and_closed_counts_refuse() {
    let mut response = response();
    let name = protocol::Name::new("Item".to_owned()).unwrap();
    response.selection = protocol::Selection::Entity(protocol::Entity {
        name: name.clone(),
        primary_key: protocol::Name::new("id".to_owned()).unwrap(),
        fields: ic_wire::List::new(vec![]).unwrap(),
        indexes: ic_wire::List::new(vec![]).unwrap(),
        types: ic_wire::List::new(vec![]).unwrap(),
        cardinality: protocol::Cardinality::Available(0),
    });
    let request = protocol::Request::Entity {
        schema: [7; 32],
        name,
        count: true,
    };
    let decode_response = |response: &protocol::Response| {
        decode(
            &frame(
                &Ok(response.clone()),
                protocol::INSPECT,
                Direction::Response,
            ),
            &request,
        )
    };
    assert!(decode_response(&response).is_err());
    response.admission = protocol::Admission::Readable {
        incarnation: [1; 16],
    };
    assert!(decode_response(&response).is_ok());
    response.schema = [8; 32];
    assert!(decode_response(&response).is_err());
}

#[test]
fn named_metadata_keeps_bounds_and_references_separate() {
    let name = |s: &str| protocol::Name::new(s.to_owned()).unwrap();
    let value = protocol::Value {
        kind: protocol::Kind::Text(4),
        optional: true,
    };
    let mut response = response();
    response.selection = protocol::Selection::Entity(protocol::Entity {
        name: name("Item"),
        primary_key: name("id"),
        fields: ic_wire::List::new(vec![protocol::Field {
            name: name("owner"),
            value: value.clone(),
            reference: Some(protocol::Reference {
                target: name("User"),
                index: name("owner,id"),
            }),
        }])
        .unwrap(),
        indexes: ic_wire::List::new(vec![]).unwrap(),
        types: ic_wire::List::new(vec![
            protocol::Definition {
                name: name("Nested"),
                body: protocol::Body::Record(
                    ic_wire::List::new(vec![protocol::Field {
                        name: name("next"),
                        value: protocol::Value {
                            kind: protocol::Kind::Named(name("Nested")),
                            optional: true,
                        },
                        reference: None,
                    }])
                    .unwrap(),
                ),
            },
            protocol::Definition {
                name: name("Choice"),
                body: protocol::Body::Enum(
                    ic_wire::List::new(vec![protocol::Variant {
                        name: name("Some"),
                        payload: Some(value),
                    }])
                    .unwrap(),
                ),
            },
            protocol::Definition {
                name: name("Tags"),
                body: protocol::Body::Set {
                    item: protocol::Kind::Text(8),
                    max_items: 3,
                },
            },
            protocol::Definition {
                name: name("Data"),
                body: protocol::Body::Map {
                    key: protocol::Kind::Unsigned(8),
                    value: protocol::Value {
                        kind: protocol::Kind::List {
                            item: Box::new(protocol::Kind::Blob(4)),
                            max_items: 2,
                        },
                        optional: false,
                    },
                    max_items: 5,
                },
            },
        ])
        .unwrap(),
        cardinality: protocol::Cardinality::Unavailable,
    });
    let rendered = render(&response);
    for expected in [
        "Info",
        "Reference target",
        "Reference index",
        "User",
        "owner,id",
        "max 4 UTF-8 bytes",
        "Nested",
        "Choice",
        "Tags",
        "Data",
        "List<Blob>",
        "max 2 items; max 4 bytes",
        "Unavailable",
    ] {
        assert!(
            rendered.contains(expected),
            "missing {expected}: {rendered}"
        );
    }
    assert_eq!(
        kind(&protocol::Kind::Named(name("bad\nname"))).0,
        "bad\\nname"
    );
}
