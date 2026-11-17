use super::{MAINNET_ROOT_SUBNET, Network};
use crate::Error;

#[test]
fn historical_local_binding_retains_bytes_but_cannot_dispatch_without_trust() {
    let original = format!(
        r#"{{"kind":"Local","url":"http://127.0.0.1:4943/","root_key":"{}","routing_canister":"rrkah-fqaaa-aaaaa-aaaaq-cai"}}"#,
        crate::hex(include_bytes!("mainnet-root.der"))
    );
    let network: Network = serde_json::from_str(&original).unwrap();
    assert_eq!(serde_json::to_string(&network).unwrap(), original);
    let old_digest = network.binding_id().unwrap();
    assert_eq!(old_digest, crate::artifact::digest(original.as_bytes()));
    assert!(
        matches!(network.client(), Err(Error::Blocked(message)) if message.contains("root_subnet"))
    );
    let Network::Local {
        url,
        root_key,
        routing_canister,
        ..
    } = network
    else {
        unreachable!()
    };
    let admitted = Network::Local {
        url,
        root_key,
        routing_canister,
        root_subnet: Some(MAINNET_ROOT_SUBNET.into()),
    };
    assert_ne!(admitted.binding_id().unwrap(), old_digest);
    assert!(admitted.client().is_ok());
}

#[test]
fn transport_requires_a_valid_key_and_explicit_subnet_identity() {
    assert!(Network::Mainnet.client().is_ok());
    for subnet in ["2vxsx-fae", "aaaaa-aa", "not a principal"] {
        let network = Network::Local {
            url: "http://127.0.0.1:4943/".into(),
            root_key: crate::hex(include_bytes!("mainnet-root.der")),
            root_subnet: Some(subnet.into()),
            routing_canister: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
        };
        assert!(network.client().is_err());
    }
    let invalid_key = Network::Local {
        url: "http://127.0.0.1:4943/".into(),
        root_key: "00".repeat(133),
        root_subnet: Some(MAINNET_ROOT_SUBNET.into()),
        routing_canister: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
    };
    assert!(invalid_key.client().is_err());
}

#[test]
fn certificate_time_preserves_legacy_staleness_and_future_policy() {
    let (expiry, policy) = super::context().unwrap();
    for window in [policy.certificate, policy.delegation] {
        assert_eq!(window.max_age_ns, 300_000_000_000);
        assert_eq!(
            window.reference_ns.checked_add(window.max_future_skew_ns),
            Some(u64::MAX)
        );
        assert_eq!(
            window.reference_ns.checked_add(300_000_000_000),
            Some(expiry)
        );
    }
}
