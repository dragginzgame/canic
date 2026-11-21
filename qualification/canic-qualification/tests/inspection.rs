//! Compare host ABI declarations with each actual User Directory executable.
use canic::inspection::{Schemas, methods};
use canic_asset_catalog as _;
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

#[test]
fn supplier_mounts_register_their_public_wire_contracts() {
    use canic::inspection::{Declaration, inventory};

    for (actor, name, mode, id) in [
        (
            "canic_asset_catalog",
            "blob_update",
            "update",
            canic::blob::UPDATE,
        ),
        (
            "canic_asset_catalog",
            "blob_query",
            "query",
            canic::blob::QUERY,
        ),
        (
            "canic_asset_catalog",
            "blob_management",
            "update",
            canic::blob::MANAGEMENT,
        ),
        (
            "canic",
            "metrics",
            "query",
            <canic::metrics::ReadMetrics as canic::method::Contract>::ID,
        ),
    ] {
        let mut declarations = inventory::iter::<Declaration>
            .into_iter()
            .filter(|d| d.module.split("::").next() == Some(actor) && d.name == name);
        let declaration = declarations.next().expect(name);
        assert_eq!((declaration.mode, declaration.id), (mode, id), "{name}");
        assert!(declarations.next().is_none(), "duplicate {name}");
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

mod role_layout {
    use canic::method::{Contract, wire};
    use canic::topology::attestation::RoleMethod;

    // This package does not depend on the receiver crate or its VISITS state.
    mod protocol {
        use canic::attested_method as query;
        include!("../../canic-directory-witness/src/visit_count.rs");
    }

    #[test]
    fn reads_the_shared_query_descriptor_without_linking_the_receiver() {
        type Query = protocol::visits;
        assert_eq!(Query::NAME, "visits");
        assert_eq!(Query::ID, wire::method_id("visits"));
        assert_eq!(Query::ROLES, &["user_shard"]);
        assert_eq!(Query::REQUEST.bytes, canic::method::LIMITS.bytes);
        assert_eq!(Query::REPLY.bytes, Query::REQUEST.bytes);
        let frame = |direction| wire::Encoding::Frame {
            method: wire::method_id("visits"),
            direction,
        };
        assert_eq!(
            Query::encode_request(&()).unwrap(),
            wire::encode(&(), frame(wire::Direction::Request), canic::method::LIMITS).unwrap()
        );
        let reply = wire::encode(
            &7u64,
            frame(wire::Direction::Response),
            canic::method::LIMITS,
        )
        .unwrap();
        assert_eq!(Query::encode_reply(&7u64).unwrap(), reply);
        assert_eq!(Query::decode_reply(&reply).unwrap(), 7);
    }

    struct Explicit;
    impl Contract for Explicit {
        const NAME: &'static str = "explicit_lookup";
        const ID: u64 = wire::method_id("inline_lookup");
        const REPLY: wire::Limits = wire::Limits {
            bytes: 2048,
            ..Self::REQUEST
        };
        type Input = u64;
        type Output = Option<String>;
    }
    impl RoleMethod for Explicit {
        const ROLES: &'static [&'static str] = &["project"];
    }

    #[canic::query(attested_method = Explicit)]
    fn explicit_lookup(_: u64) -> Option<String> {
        None
    }

    #[canic::query(roles = Explicit::ROLES, reply_limits = Explicit::REPLY)]
    fn inline_lookup(_: u64) -> Option<String> {
        None
    }

    #[test]
    fn equivalent_attested_queries_report_the_same_effective_abi() {
        let schemas = super::Schemas::default();
        let declared = |name| {
            let declaration = canic::inspection::inventory::iter::<canic::inspection::Declaration>
                .into_iter()
                .find(|d| d.module == module_path!() && d.name == name)
                .unwrap();
            (
                declaration.id,
                declaration.mode,
                serde_json::to_value((declaration.request)(&schemas).unwrap()).unwrap(),
                serde_json::to_value((declaration.reply)(&schemas).unwrap()).unwrap(),
                declaration.request_limits,
                declaration.reply_limits,
            )
        };
        let inline = declared("inline_lookup");
        assert_eq!(inline.0, Explicit::ID);
        assert_eq!(inline.1, "query");
        assert!(
            inline.2["rust"]
                .as_str()
                .unwrap()
                .contains("attestation::Claim")
        );
        assert_eq!(inline.4.bytes, Explicit::REQUEST.bytes);
        assert_eq!(inline.5.bytes, 2048);
        assert_eq!(inline, declared("explicit_lookup"));
    }
}
