//! Exact Blob installation decoding for dedicated and nested application inputs.
//! Blob owns Wire limits; the embedding application owns its outer envelope.

use canic_blob_service::__private::ic_blob_storage_contracts::wire;
use canic_blob_service::dto::configuration::{
    ServiceBillingInput, ServiceConfigurationInput, ServiceFundingInput, ServiceInstallationInput,
    ServiceResourceInput,
};
use canic_contracts::protocol::initialization::encode;
use ic_platform::principal::ExternalPrincipal as Principal;

#[derive(Debug, serde::Deserialize, PartialEq, serde::Serialize)]
struct EmbeddedInput {
    initial_count: u64,
    blob: Vec<u8>,
}

#[derive(serde::Serialize)]
struct UnmodeledInstallationInput {
    configuration: ServiceConfigurationInput,
    project: String,
    unmodeled: Principal,
}

#[derive(serde::Serialize)]
struct UnmodeledEmbeddedInput {
    initial_count: u64,
    blob: Vec<u8>,
    unmodeled: Principal,
}

fn installation() -> ServiceInstallationInput {
    let principal = |byte| Principal::from_slice(&[byte; 29]);
    ServiceInstallationInput {
        configuration: ServiceConfigurationInput {
            service: principal(1).into(),
            operator: principal(7).into(),
            payment_account: principal(8).into(),
            namespace: 7,
            resources: ServiceResourceInput {
                max_tenants: 2,
                max_object_bytes: 10,
                max_headers: 8,
                max_header_bytes: 1024,
                max_chunks: 4,
                max_tenant_chunks: 2,
                max_objects: 4,
                max_tenant_objects: 2,
                max_physical_bytes: 30_000,
                max_liability_bytes: 20_000,
                max_tenant_logical_bytes: 10_000,
                max_references_per_object: 2,
                max_receipts_per_object: 3,
                max_active: 4,
                max_tenant_active: 2,
            },
            billing: ServiceBillingInput {
                cashier: principal(10).into(),
                reserve: 100,
                minimum_balance: 100,
                target_balance: 200,
                max_gateway_entries: 8,
                max_gateway_unique: 4,
            },
            funding: ServiceFundingInput {
                allocated: 10_000,
                renewal_ceiling: 10_000,
                reserve: 100,
                max_attempts: 4,
            },
        },
        project: "decoder-test".into(),
    }
}

fn unmodeled(input: &ServiceInstallationInput) -> UnmodeledInstallationInput {
    UnmodeledInstallationInput {
        configuration: input.configuration,
        project: input.project.clone(),
        unmodeled: Principal::anonymous(),
    }
}

#[test]
fn dedicated_configuration_is_exact_and_bounded_wire() {
    let input = installation();
    let current = wire::encode_request("blob_install", &input).unwrap();
    assert_eq!(
        wire::decode_request::<ServiceInstallationInput>(
            "blob_install",
            &current,
            wire::request_limit("blob_install").unwrap()
        ),
        Ok(input.clone())
    );
    for wrong_frame in [
        wire::encode_request("blob_post_upgrade", &input).unwrap(),
        wire::encode_response("blob_install", &input).unwrap(),
    ] {
        assert_eq!(
            wire::decode_request::<ServiceInstallationInput>(
                "blob_install",
                &wrong_frame,
                wire::request_limit("blob_install").unwrap()
            ),
            Err(wire::Error::Identity)
        );
    }
    let field = wire::encode_request("blob_install", unmodeled(&input)).unwrap();
    assert_eq!(
        wire::decode_request::<ServiceInstallationInput>(
            "blob_install",
            &field,
            wire::request_limit("blob_install").unwrap()
        ),
        Err(wire::Error::Trailing)
    );
    let mut extra = current.clone();
    extra.push(0xf6);
    assert_eq!(
        wire::decode_request::<ServiceInstallationInput>(
            "blob_install",
            &extra,
            wire::request_limit("blob_install").unwrap()
        ),
        Err(wire::Error::Trailing)
    );
    assert_eq!(
        wire::decode_request::<ServiceInstallationInput>(
            "blob_install",
            &current,
            current.len() - 1
        ),
        Err(wire::Error::Limit)
    );
}

#[test]
fn embedded_configuration_bounds_its_own_envelope_and_nested_blob_bytes() {
    let envelope_limits = ic_wire::Limits {
        bytes: 16_384,
        allocation: 262_144,
        nodes: 16_384,
        depth: 32,
    };
    let blob = installation();
    let input = EmbeddedInput {
        initial_count: 41,
        blob: wire::encode_request("blob_install", &blob).unwrap(),
    };
    let current = encode(&input).unwrap();
    let (decoded,): (EmbeddedInput,) = ic_wire::native::decode(&current, envelope_limits).unwrap();
    assert_eq!(decoded, input);
    assert_eq!(
        wire::decode_request::<ServiceInstallationInput>(
            "blob_install",
            &decoded.blob,
            wire::request_limit("blob_install").unwrap()
        ),
        Ok(blob.clone())
    );
    let field = encode(UnmodeledEmbeddedInput {
        initial_count: input.initial_count,
        blob: input.blob.clone(),
        unmodeled: Principal::anonymous(),
    })
    .unwrap();
    let extra = encode((&input, "small")).unwrap();
    for bytes in [field, extra] {
        assert_eq!(
            ic_wire::native::decode::<(EmbeddedInput,)>(&bytes, envelope_limits),
            Err(ic_wire::Error::Trailing)
        );
    }
    let nested = encode(EmbeddedInput {
        initial_count: 41,
        blob: wire::encode_request("blob_install", unmodeled(&blob)).unwrap(),
    })
    .unwrap();
    let (decoded,): (EmbeddedInput,) = ic_wire::native::decode(&nested, envelope_limits).unwrap();
    assert_eq!(
        wire::decode_request::<ServiceInstallationInput>(
            "blob_install",
            &decoded.blob,
            wire::request_limit("blob_install").unwrap()
        ),
        Err(wire::Error::Trailing)
    );
}
