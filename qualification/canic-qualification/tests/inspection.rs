//! Compare host ABI declarations with each actual User Directory executable.
use canic::inspection::{Schemas, methods};
use ex_user_directory_hub as _;
use ex_user_directory_shard as _;
use std::path::Path;

#[test]
fn inventory_matches_built_hub_and_shard() {
    for role in ["hub", "shard"] {
        let actor = format!("ex_user_directory_{role}");
        let inventory = methods(&actor, &Schemas::default()).unwrap();
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../apps/examples/user-directory/artifacts/user_{role}.wasm"
        ));
        let exports =
            canic_host::endpoints(&path).expect("inspect the built user-directory canister");
        assert_eq!(
            exports
                .iter()
                .map(|e| (e.name.as_str(), e.mode))
                .collect::<Vec<_>>(),
            inventory
                .iter()
                .map(|(&name, m)| (name, m.mode))
                .collect::<Vec<_>>()
        );
        assert_eq!(inventory["readiness"].id, canic::readiness::METHOD);
        assert_eq!(
            inventory["portable_session_read"].id,
            canic::auth::protocol::ReadSession::ID
        );
        if role == "hub" {
            assert_eq!(inventory["my_shard"].request.rust, "()");
        } else {
            assert_eq!(
                inventory["provision_user"].id,
                ic_wire::method_id("provision_user")
            );
            assert!(
                inventory["provision_user"]
                    .request
                    .schema
                    .to_string()
                    .contains("binding")
            );
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
enum Legacy {
    First,
    Last,
}

#[derive(ic_wire::typescript::Wire)]
enum Revised {
    First,
    Inserted,
    Last,
}

#[derive(ic_wire::typescript::Wire)]
enum Original {
    First,
    Last,
}

#[derive(serde::Deserialize, serde::Serialize, ic_wire::typescript::Wire)]
enum Filter {
    All,
    Any(Vec<Filter>),
}

#[canic::query]
fn nested_reply(_filter: Filter) -> Option<Legacy> {
    None
}

#[test]
fn nested_reply_changes_the_manifest_and_missing_evidence_names_the_method() {
    let actor = env!("CARGO_CRATE_NAME");
    let mut schemas = Schemas::default();
    let error = methods(actor, &schemas).err().unwrap();
    assert!(
        error.contains("nested_reply reply: no qualified schema"),
        "{error}"
    );
    schemas.describe_as::<Option<Legacy>, Option<Original>>();
    let original = methods(actor, &schemas).unwrap();
    schemas.describe_as::<Option<Legacy>, Option<Revised>>();
    let revised = methods(actor, &schemas).unwrap();
    assert_eq!(original["nested_reply"].id, revised["nested_reply"].id);
    assert_eq!(
        original["nested_reply"].request.schema,
        revised["nested_reply"].request.schema
    );
    assert!(
        original["nested_reply"]
            .request
            .schema
            .to_string()
            .contains("Any")
    );
    assert_eq!(
        serde_json::to_value(&original["nested_reply"]).unwrap()["id"],
        format!("0x{:016x}", ic_wire::method_id("nested_reply"))
    );
    let before = &original["nested_reply"].reply.schema;
    let after = &revised["nested_reply"].reply.schema;
    let last_tag = |schema: &serde_json::Value| {
        schema
            .as_object()
            .unwrap()
            .values()
            .filter_map(|node| node["variants"].as_array())
            .flatten()
            .find(|variant| variant["name"] == "Last")
            .unwrap()["id"]
            .as_u64()
            .unwrap()
    };
    assert_eq!(last_tag(before), 1);
    assert_eq!(last_tag(after), 2);
    assert!(!before.to_string().contains("Inserted"));
    assert!(after.to_string().contains("Inserted"));
    // Construct every witness variant so the test also type-checks its declared layout.
    let _ = (
        Original::First,
        Original::Last,
        Revised::First,
        Revised::Inserted,
        Revised::Last,
    );
}
