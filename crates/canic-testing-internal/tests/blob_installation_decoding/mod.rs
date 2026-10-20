//! Exact Blob installation decoding for dedicated and nested application inputs.
//! The adapter owns limits; these native cases perform no installation effects.

use canic::endpoint::ArgumentDecodeError;
use canic_blob_service::__private::ic_blob_storage_contracts::wire;
use canic_blob_service::{
    dto::configuration::{
        ServiceBillingInput, ServiceConfigurationInput, ServiceFundingInput,
        ServiceInstallationInput, ServiceReadInput, ServiceResourceInput,
    },
    lifecycle::CONFIGURATION_LIMITS,
};
use ic_platform::{candid::contract::CandidType, principal::ExternalPrincipal as Principal};

#[derive(CandidType, Debug, serde::Deserialize, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
struct EmbeddedInput {
    initial_count: u64,
    blob: Vec<u8>,
}

#[derive(serde::Serialize)]
struct UnmodeledInstallationInput {
    configuration: ServiceConfigurationInput,
    project: String,
    completion_verifier: Principal,
    unmodeled: Principal,
}

#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
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
            reads: ServiceReadInput {
                sessions: 2,
                tenant_sessions: 1,
                reply_bytes: 2048,
                bytes: 4096,
                tenant_bytes: 2048,
            },
        },
        project: "decoder-test".into(),
        completion_verifier: principal(11).into(),
    }
}

fn unmodeled(input: &ServiceInstallationInput) -> UnmodeledInstallationInput {
    UnmodeledInstallationInput {
        configuration: input.configuration,
        project: input.project.clone(),
        completion_verifier: input.completion_verifier.into(),
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
            CONFIGURATION_LIMITS.max_bytes
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
                CONFIGURATION_LIMITS.max_bytes
            ),
            Err(wire::Error::Identity)
        );
    }
    let field = wire::encode_request("blob_install", unmodeled(&input)).unwrap();
    assert_eq!(
        wire::decode_request::<ServiceInstallationInput>(
            "blob_install",
            &field,
            CONFIGURATION_LIMITS.max_bytes
        ),
        Err(wire::Error::Trailing)
    );
    let mut extra = current.clone();
    extra.push(0xf6);
    assert_eq!(
        wire::decode_request::<ServiceInstallationInput>(
            "blob_install",
            &extra,
            CONFIGURATION_LIMITS.max_bytes
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
    let blob = installation();
    let input = EmbeddedInput {
        initial_count: 41,
        blob: wire::encode_request("blob_install", &blob).unwrap(),
    };
    let current = ic_platform::candid::contract::encode_one(&input).unwrap();
    let (decoded,): (EmbeddedInput,) = CONFIGURATION_LIMITS.decode(&current).unwrap();
    assert_eq!(decoded, input);
    assert_eq!(
        wire::decode_request::<ServiceInstallationInput>(
            "blob_install",
            &decoded.blob,
            CONFIGURATION_LIMITS.max_bytes
        ),
        Ok(blob.clone())
    );
    let field = ic_platform::candid::contract::encode_one(UnmodeledEmbeddedInput {
        initial_count: input.initial_count,
        blob: input.blob.clone(),
        unmodeled: Principal::anonymous(),
    })
    .unwrap();
    let extra = ic_platform::candid::contract::encode_args((&input, "small")).unwrap();
    for bytes in [field, extra] {
        assert_eq!(
            CONFIGURATION_LIMITS.decode::<(EmbeddedInput,)>(&bytes),
            Err(ArgumentDecodeError::InvalidCandid)
        );
    }
    let nested = ic_platform::candid::contract::encode_one(EmbeddedInput {
        initial_count: 41,
        blob: wire::encode_request("blob_install", unmodeled(&blob)).unwrap(),
    })
    .unwrap();
    let (decoded,): (EmbeddedInput,) = CONFIGURATION_LIMITS.decode(&nested).unwrap();
    assert_eq!(
        wire::decode_request::<ServiceInstallationInput>(
            "blob_install",
            &decoded.blob,
            CONFIGURATION_LIMITS.max_bytes
        ),
        Err(wire::Error::Trailing)
    );
}
