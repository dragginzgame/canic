//! Generate the browser issuer codecs from the actual mounted Rust contracts.
use canic::auth::{
    issuer::{self, Issue, Logout, Lookup, Refusal, Renew, Revoked, Status},
    protocol::{
        CredentialEnvelope, Logout as ReceiverLogout,
        model::{LogoutResult, PortableCredential},
    },
};
use ic_auth_wire::wire;
use std::{fs, path::Path};
use wire::{
    Bytes, Limits, List, Text,
    typescript::{self, Method, PayloadLimits, SchemaLimits, SchemaRegistry, Wire},
};

fn payload(limits: Limits) -> PayloadLimits {
    PayloadLimits {
        bytes: (limits.bytes - 11) as u64,
        nodes: limits.nodes as u64,
        depth: limits.depth as u64,
    }
}

fn module<Q: Wire, R: Wire>(id: u64, request: Limits, response: Limits) -> String {
    let mut registry = SchemaRegistry::new(SchemaLimits::default()).unwrap();
    registry.register::<Q>("Request").unwrap();
    registry.register::<R>("Response").unwrap();
    registry
        .finish()
        .unwrap()
        .method_module(
            &[Method {
                export: "codec",
                id,
                request: "Request",
                response: "Response",
                request_limits: payload(request),
                response_limits: payload(response),
            }],
            "./wire-runtime.js",
            &Default::default(),
            |report| eprint!("{report}"),
        )
        .unwrap()
}

fn main() -> std::io::Result<()> {
    let directory = std::env::args_os()
        .nth(1)
        .expect("generated output directory");
    let directory = Path::new(&directory);
    fs::create_dir_all(directory)?;
    let request = issuer::REQUEST_LIMITS;
    let response = issuer::RESPONSE_LIMITS;
    for (name, source) in [
        (
            "issue",
            module::<Issue, Result<Status, Refusal>>(issuer::ISSUE, request, response),
        ),
        (
            "renew",
            module::<Renew, Result<Status, Refusal>>(issuer::RENEW, request, response),
        ),
        (
            "reconcile",
            module::<Lookup, Result<Status, Refusal>>(issuer::RECONCILE, request, response),
        ),
        (
            "logout",
            module::<Logout, Result<Revoked, Refusal>>(issuer::LOGOUT, request, response),
        ),
        (
            "credential",
            module::<PortableCredential, PortableCredential>(
                CredentialEnvelope::ID,
                CredentialEnvelope::REQUEST_LIMITS,
                CredentialEnvelope::RESPONSE_LIMITS,
            ),
        ),
        (
            "receiver-logout",
            module::<(), LogoutResult>(
                ReceiverLogout::ID,
                ReceiverLogout::REQUEST_LIMITS,
                ReceiverLogout::RESPONSE_LIMITS,
            ),
        ),
    ] {
        fs::write(directory.join(format!("{name}.ts")), source)?;
    }
    fs::write(directory.join("wire-runtime.ts"), typescript::RUNTIME)?;
    // Native-produced bytes are consumed by the browser tests, independently of
    // the generated JS encoder/decoder. Synthetic proofs establish codec parity only.
    let input = Issue {
        epoch: 1,
        operation_id: [2; 32],
        login_epoch: 3,
        login_id: [4; 32],
        browser_public_key: [5; 32],
        grants: List::new(vec![canic::auth::protocol::model::DelegatedRoleGrant {
            target: Text::new("profile".into()).unwrap(),
            scopes: List::new(vec![Text::new("read".into()).unwrap()]).unwrap(),
        }])
        .unwrap(),
    };
    let credential = PortableCredential {
        claims: Bytes::new(vec![1, 2, 3]).unwrap(),
        proof: canic::auth::protocol::model::PortableProof::Ed25519(
            Bytes::new(vec![4; 64]).unwrap(),
        ),
    };
    let reply = Ok::<_, Refusal>(Status::Issued {
        credential,
        admission_deadline_ns: 900,
    });
    let request_bytes = wire::encode(
        &input,
        wire::Encoding::Frame {
            method: issuer::ISSUE,
            direction: wire::Direction::Request,
        },
        request,
    )
    .unwrap();
    let reply_bytes = wire::encode(
        &reply,
        wire::Encoding::Frame {
            method: issuer::ISSUE,
            direction: wire::Direction::Response,
        },
        response,
    )
    .unwrap();
    fs::write(
        directory.join("native-vectors.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"request": request_bytes, "response": reply_bytes}),
        )
        .unwrap(),
    )
}
