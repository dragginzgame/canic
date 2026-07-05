use super::merkle::chain_key_batch_node_hash;
use super::selection::chain_key_template_due;
use super::*;
use crate::{
    domain::policy::pure::auth::{
        RootDelegatedRoleGrantPolicy, RootDelegationAudiencePolicy, RootIssuerPolicy,
        RootIssuerRenewalOutcome, RootIssuerRenewalTemplate,
    },
    dto::auth::{ChainKeyAlgorithm, ChainKeyBatchWitnessStepV1, ChainKeyKeyId},
    ids::{BuildNetwork, CanisterRole},
    ops::auth::delegated::chain_key::{
        ChainKeyRootVerifierPolicy, ChainKeySignatureVerificationInput,
        verify_chain_key_batch_root_proof, verify_chain_key_ecdsa_signature,
    },
    ops::{
        auth::delegated::chain_key_signing::ChainKeySignerFuture,
        ic::mgmt::{
            EcdsaPublicKeyArgs, EcdsaPublicKeyResult, SignWithEcdsaArgs, SignWithEcdsaResult,
        },
    },
};
use futures::executor::block_on;
use k256::ecdsa::{
    Signature as K256TestSignature, SigningKey as K256SigningKey, signature::hazmat::PrehashSigner,
};

fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

fn signing_key() -> K256SigningKey {
    K256SigningKey::from_slice(&[7; 32]).expect("test signing key should parse")
}

fn signing_policy() -> ChainKeySigningPolicy {
    ChainKeySigningPolicy {
        root_canister_id: p(1),
        algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
        key_id: ChainKeyKeyId {
            name: "test_key_1".to_string(),
        },
        derivation_path: vec![b"canic".to_vec(), b"delegation".to_vec()],
        public_key: signing_key()
            .verifying_key()
            .to_encoded_point(true)
            .as_bytes()
            .to_vec(),
        key_version: 4,
        build_network: BuildNetwork::Local,
        allow_test_chain_key: true,
    }
}

fn policy(issuer_pid: Principal) -> RootIssuerPolicy {
    RootIssuerPolicy {
        issuer_pid,
        enabled: true,
        allowed_audiences: vec![RootDelegationAudiencePolicy::Project("test".to_string())],
        allowed_grants: vec![RootDelegatedRoleGrantPolicy {
            target: CanisterRole::owned("project_instance".to_string()),
            scopes: vec!["read".to_string()],
        }],
        max_cert_ttl_ns: 120_000_000_000,
        refresh_after_ratio_bps: 8_000,
    }
}

fn template(issuer_pid: Principal, cert_ttl_ns: u64) -> RootIssuerRenewalTemplate {
    RootIssuerRenewalTemplate {
        issuer_pid,
        enabled: true,
        audience: RootDelegationAudiencePolicy::Project("test".to_string()),
        grants: vec![RootDelegatedRoleGrantPolicy {
            target: CanisterRole::owned("project_instance".to_string()),
            scopes: vec!["read".to_string()],
        }],
        cert_ttl_ns,
    }
}

fn input(signing_policy: &ChainKeySigningPolicy) -> PrepareDueChainKeyRootDelegationBatchInput<'_> {
    PrepareDueChainKeyRootDelegationBatchInput {
        signing_policy,
        max_cert_ttl_ns: 120_000_000_000,
        max_revocation_latency_ns: 60_000_000_000,
        min_accepted_proof_epoch: 10,
        registry_epoch: 11,
        registry_hash: [22; 32],
        required_issuer_pid: None,
        now_ns: 1_000,
    }
}

fn verifier_policy(signing_policy: &ChainKeySigningPolicy) -> ChainKeyRootVerifierPolicy {
    ChainKeyRootVerifierPolicy {
        root_canister_id: signing_policy.root_canister_id,
        algorithm: signing_policy.algorithm,
        key_id: signing_policy.key_id.clone(),
        derivation_path_hash: chain_key_derivation_path_hash(&signing_policy.derivation_path),
        public_key: signing_policy.public_key.clone(),
        key_version: signing_policy.key_version,
        min_accepted_key_version: signing_policy.key_version,
        min_accepted_proof_epoch: 10,
        min_accepted_registry_epoch: 11,
        valid_from_ns: 1,
        accept_until_ns: 120_000_000_000,
        build_network: BuildNetwork::Local,
        allow_test_chain_key: true,
        max_revocation_latency_ns: 60_000_000_000,
    }
}

fn sign_header(header: &ChainKeyBatchHeaderV1) -> crate::dto::auth::ChainKeyRootSignatureV1 {
    let signature: K256TestSignature = signing_key()
        .sign_prehash(&chain_key_batch_header_hash(header))
        .expect("test prehash signature should sign");
    let policy = signing_policy();
    crate::dto::auth::ChainKeyRootSignatureV1 {
        algorithm: policy.algorithm,
        key_id: policy.key_id,
        derivation_path: policy.derivation_path,
        public_key: policy.public_key,
        signature: signature.to_bytes().to_vec(),
    }
}

struct MockSigner {
    public_key: Vec<u8>,
    signature: Vec<u8>,
    public_key_calls: usize,
    sign_calls: usize,
}

impl MockSigner {
    fn valid_for(header: &ChainKeyBatchHeaderV1) -> Self {
        let signature = sign_header(header);
        Self {
            public_key: signature.public_key,
            signature: signature.signature,
            public_key_calls: 0,
            sign_calls: 0,
        }
    }
}

impl ChainKeySigner for MockSigner {
    fn ecdsa_public_key(
        &mut self,
        _args: EcdsaPublicKeyArgs,
    ) -> ChainKeySignerFuture<'_, EcdsaPublicKeyResult> {
        self.public_key_calls += 1;
        Box::pin(async move {
            Ok(EcdsaPublicKeyResult {
                public_key: self.public_key.clone(),
                chain_code: vec![9; 32],
            })
        })
    }

    fn sign_with_ecdsa(
        &mut self,
        _args: SignWithEcdsaArgs,
    ) -> ChainKeySignerFuture<'_, SignWithEcdsaResult> {
        self.sign_calls += 1;
        Box::pin(async move {
            Ok(SignWithEcdsaResult {
                signature: self.signature.clone(),
            })
        })
    }
}

struct DynamicMockSigner {
    public_key_calls: usize,
    sign_calls: usize,
}

impl ChainKeySigner for DynamicMockSigner {
    fn ecdsa_public_key(
        &mut self,
        _args: EcdsaPublicKeyArgs,
    ) -> ChainKeySignerFuture<'_, EcdsaPublicKeyResult> {
        self.public_key_calls += 1;
        Box::pin(async move {
            Ok(EcdsaPublicKeyResult {
                public_key: signing_policy().public_key,
                chain_code: vec![9; 32],
            })
        })
    }

    fn sign_with_ecdsa(
        &mut self,
        args: SignWithEcdsaArgs,
    ) -> ChainKeySignerFuture<'_, SignWithEcdsaResult> {
        self.sign_calls += 1;
        Box::pin(async move {
            let signature: K256TestSignature = signing_key()
                .sign_prehash(&args.message_hash)
                .expect("test prehash signature should sign");
            Ok(SignWithEcdsaResult {
                signature: signature.to_bytes().to_vec(),
            })
        })
    }
}

struct StaleDuringSignSigner {
    batch_id: [u8; 32],
    public_key_calls: usize,
    sign_calls: usize,
}

impl ChainKeySigner for StaleDuringSignSigner {
    fn ecdsa_public_key(
        &mut self,
        _args: EcdsaPublicKeyArgs,
    ) -> ChainKeySignerFuture<'_, EcdsaPublicKeyResult> {
        self.public_key_calls += 1;
        Box::pin(async move {
            Ok(EcdsaPublicKeyResult {
                public_key: signing_policy().public_key,
                chain_code: vec![9; 32],
            })
        })
    }

    fn sign_with_ecdsa(
        &mut self,
        args: SignWithEcdsaArgs,
    ) -> ChainKeySignerFuture<'_, SignWithEcdsaResult> {
        self.sign_calls += 1;
        Box::pin(async move {
            let mut batch = AuthStateOps::chain_key_root_delegation_batch(self.batch_id)
                .expect("batch should still exist while signing");
            batch.status = ChainKeyRootDelegationBatchStatus::FailedRetryable;
            batch.retry_after_ns = Some(batch.header.expires_at_ns);
            batch.failure = Some("stale registry epoch or hash".to_string());
            AuthStateOps::upsert_chain_key_root_delegation_batch(batch);

            let signature: K256TestSignature = signing_key()
                .sign_prehash(&args.message_hash)
                .expect("test prehash signature should sign");
            Ok(SignWithEcdsaResult {
                signature: signature.to_bytes().to_vec(),
            })
        })
    }
}

#[test]
fn chain_key_batch_builder_prepares_merkle_batch_that_verifier_accepts() {
    let signing_policy = signing_policy();
    let issuer_a = p(42);
    let issuer_b = p(41);
    AuthStateOps::upsert_root_issuer_policy(policy(issuer_a));
    AuthStateOps::upsert_root_issuer_policy(policy(issuer_b));

    let batch = build_chain_key_root_delegation_batch(
        input(&signing_policy),
        &[
            DueChainKeyTemplate {
                template: template(issuer_a, 90_000_000_000),
            },
            DueChainKeyTemplate {
                template: template(issuer_b, 60_000_000_000),
            },
        ],
        10,
    )
    .expect("batch should build");

    assert_eq!(batch.status, ChainKeyRootDelegationBatchStatus::Prepared);
    assert_eq!(batch.issuers.len(), 2);
    assert_eq!(batch.header.not_before_ns, 1_000);
    assert_eq!(batch.header.expires_at_ns, 60_000_001_000);
    assert_eq!(
        batch.header_hash,
        chain_key_batch_header_hash(&batch.header)
    );
    assert_eq!(batch.issuers[0].issuer_pid, issuer_b);
    assert_eq!(batch.issuers[1].issuer_pid, issuer_a);

    let signature = sign_header(&batch.header);
    for issuer in &batch.issuers {
        let proof = crate::dto::auth::RootProof::IcChainKeyBatchSignatureV1(
            crate::dto::auth::IcChainKeyBatchSignatureProofV1 {
                header: batch.header.clone(),
                delegation_cert: issuer.chain_key_delegation_cert.clone(),
                issuer_witness: issuer.issuer_witness.clone(),
                signature: signature.clone(),
            },
        );

        verify_chain_key_batch_root_proof(
            crate::ops::auth::delegated::chain_key::VerifyChainKeyBatchRootProofInput {
                cert: &issuer.delegation_cert,
                root_proof: &proof,
                policy: &verifier_policy(&signing_policy),
                now_ns: 1_000,
            },
            |input: ChainKeySignatureVerificationInput<'_>| verify_chain_key_ecdsa_signature(input),
        )
        .expect("builder proof material should verify");
    }
}

#[test]
fn chain_key_batch_builder_rejects_duplicate_issuer_leaves() {
    let signing_policy = signing_policy();
    let issuer = p(43);
    AuthStateOps::upsert_root_issuer_policy(policy(issuer));

    let err = build_chain_key_root_delegation_batch(
        input(&signing_policy),
        &[
            DueChainKeyTemplate {
                template: template(issuer, 90_000_000_000),
            },
            DueChainKeyTemplate {
                template: template(issuer, 60_000_000_000),
            },
        ],
        10,
    )
    .expect_err("duplicate issuer leaves must reject");

    assert!(err.to_string().contains("duplicate issuer"));
}

#[test]
fn chain_key_batch_due_template_cap_limits_one_batch_to_sixty_four_issuers() {
    let mut due_templates = (0..=MAX_CHAIN_KEY_ROOT_DELEGATION_BATCH_ISSUERS)
        .map(|index| DueChainKeyTemplate {
            template: template(
                p(u8::try_from(100 + index).expect("test issuer id should fit in u8")),
                60_000_000_000,
            ),
        })
        .collect::<Vec<_>>();
    let excluded_issuer = due_templates
        .last()
        .expect("test should include one issuer over the cap")
        .template
        .issuer_pid;

    cap_due_chain_key_templates(&mut due_templates);

    assert_eq!(
        due_templates.len(),
        MAX_CHAIN_KEY_ROOT_DELEGATION_BATCH_ISSUERS
    );
    assert!(
        due_templates
            .iter()
            .all(|due| due.template.issuer_pid != excluded_issuer)
    );
}

#[test]
fn chain_key_batch_prepare_rejects_new_batch_when_pending_quota_is_full() {
    AuthStateOps::prune_chain_key_root_delegation_batches(u64::MAX);
    let signing_policy = signing_policy();
    let issuer = p(190);
    AuthStateOps::upsert_root_issuer_policy(policy(issuer));
    AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));

    let mut stale_input = input(&signing_policy);
    stale_input.registry_hash = [99; 32];
    let base_batch = build_chain_key_root_delegation_batch(
        stale_input,
        &[DueChainKeyTemplate {
            template: template(issuer, 60_000_000_000),
        }],
        10,
    )
    .expect("base batch should build for quota fixture");

    for index in 0..MAX_PENDING_CHAIN_KEY_ROOT_DELEGATION_BATCHES {
        let mut batch = base_batch.clone();
        let id_byte = u8::try_from(index).expect("quota fixture index should fit in u8");
        batch.batch_id = [id_byte; 32];
        batch.header.batch_id = batch.batch_id;
        batch.prepared_at_ns = u64::try_from(index).expect("quota fixture index fits u64");
        batch.status = ChainKeyRootDelegationBatchStatus::Prepared;
        AuthStateOps::upsert_chain_key_root_delegation_batch(batch);
    }

    let err = prepare_due_chain_key_root_delegation_batch(input(&signing_policy))
        .expect_err("full pending state must reject a new chain-key batch");

    assert!(err.is_public_resource_exhausted());
    assert!(err.to_string().contains("quota exceeded"));
    assert!(err.to_string().contains("max_pending_batches=128"));
    AuthStateOps::prune_chain_key_root_delegation_batches(u64::MAX);
}

#[test]
fn chain_key_batch_prepare_reuses_in_flight_batch() {
    let signing_policy = signing_policy();
    let issuer = p(50);
    AuthStateOps::upsert_root_issuer_policy(policy(issuer));
    AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));

    let first = prepare_due_chain_key_root_delegation_batch(input(&signing_policy))
        .expect("first prepare should build");
    let second = prepare_due_chain_key_root_delegation_batch(input(&signing_policy))
        .expect("second prepare should reuse");

    assert!(first.batch_id.is_some());
    assert_eq!(first.prepared_issuers, 1);
    assert!(!first.reused_in_flight);
    assert_eq!(second.batch_id, first.batch_id);
    assert_eq!(second.prepared_issuers, 1);
    assert!(second.reused_in_flight);
    assert_eq!(AuthStateOps::chain_key_root_delegation_batches().len(), 1);
    assert!(AuthStateOps::chain_key_root_delegation_batch(first.batch_id.unwrap()).is_some());
}

#[test]
fn chain_key_batch_signing_signs_prepared_batch_once_and_reuses_signed_state() {
    let signing_policy = signing_policy();
    let issuer = p(51);
    AuthStateOps::upsert_root_issuer_policy(policy(issuer));
    AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));
    let prepared = prepare_due_chain_key_root_delegation_batch(input(&signing_policy))
        .expect("prepare should build a batch");
    let batch = AuthStateOps::chain_key_root_delegation_batch(prepared.batch_id.unwrap())
        .expect("prepared batch should be stored");
    let mut signer = MockSigner::valid_for(&batch.header);

    let signing_result = block_on(sign_next_chain_key_root_delegation_batch(
        &signing_policy,
        2_000,
        &mut signer,
    ))
    .expect("signing should succeed");

    assert_eq!(signing_result.batch_id, prepared.batch_id);
    assert!(signing_result.signed);
    assert!(!signing_result.reused_signed);
    assert!(!signing_result.signing_in_flight);
    assert_eq!(signer.public_key_calls, 1);
    assert_eq!(signer.sign_calls, 1);
    let stored = AuthStateOps::chain_key_root_delegation_batch(prepared.batch_id.unwrap())
        .expect("signed batch should remain stored");
    assert_eq!(stored.status, ChainKeyRootDelegationBatchStatus::Signed);
    assert!(stored.signature.is_some());
    assert_eq!(stored.signed_at_ns, Some(2_000));

    let mut second_signer = MockSigner::valid_for(&stored.header);
    let reused = block_on(sign_next_chain_key_root_delegation_batch(
        &signing_policy,
        3_000,
        &mut second_signer,
    ))
    .expect("signed batch should be reused");

    assert_eq!(reused.batch_id, prepared.batch_id);
    assert!(!reused.signed);
    assert!(reused.reused_signed);
    assert_eq!(second_signer.public_key_calls, 0);
    assert_eq!(second_signer.sign_calls, 0);
}

#[test]
fn chain_key_batch_signing_covers_multiple_issuers_with_one_signature() {
    let signing_policy = signing_policy();
    let issuer_a = p(56);
    let issuer_b = p(57);
    AuthStateOps::upsert_root_issuer_policy(policy(issuer_a));
    AuthStateOps::upsert_root_issuer_policy(policy(issuer_b));
    AuthStateOps::upsert_root_issuer_renewal_template(template(issuer_a, 60_000_000_000));
    AuthStateOps::upsert_root_issuer_renewal_template(template(issuer_b, 60_000_000_000));

    let prepared = prepare_due_chain_key_root_delegation_batch(input(&signing_policy))
        .expect("prepare should build one multi-issuer batch");
    let batch_id = prepared.batch_id.expect("prepare should return a batch id");
    assert_eq!(prepared.prepared_issuers, 2);
    let batch = AuthStateOps::chain_key_root_delegation_batch(batch_id)
        .expect("prepared multi-issuer batch should be stored");
    assert_eq!(batch.issuers.len(), 2);

    let mut signer = MockSigner::valid_for(&batch.header);
    let signing_result = block_on(sign_next_chain_key_root_delegation_batch(
        &signing_policy,
        2_000,
        &mut signer,
    ))
    .expect("multi-issuer batch signing should succeed");

    assert_eq!(signing_result.batch_id, Some(batch_id));
    assert!(signing_result.signed);
    assert_eq!(signer.public_key_calls, 1);
    assert_eq!(signer.sign_calls, 1);

    let plan = start_chain_key_root_delegation_batch_install(batch_id, 3_000)
        .expect("install plan should build")
        .expect("signed multi-issuer batch should produce an install plan");

    assert_eq!(plan.batch_id, batch_id);
    assert_eq!(plan.proofs.len(), 2);
    let proof_issuers = plan
        .proofs
        .iter()
        .map(|proof| proof.issuer_pid)
        .collect::<Vec<_>>();
    assert!(proof_issuers.contains(&issuer_a));
    assert!(proof_issuers.contains(&issuer_b));
    for proof in &plan.proofs {
        verify_chain_key_batch_root_proof(
            crate::ops::auth::delegated::chain_key::VerifyChainKeyBatchRootProofInput {
                cert: &proof.proof.cert,
                root_proof: &proof.proof.root_proof,
                policy: &verifier_policy(&signing_policy),
                now_ns: 3_000,
            },
            |input: ChainKeySignatureVerificationInput<'_>| verify_chain_key_ecdsa_signature(input),
        )
        .expect("each issuer proof from the shared batch should verify");
    }

    let mut reused_signer = MockSigner::valid_for(&batch.header);
    let reused = block_on(sign_next_chain_key_root_delegation_batch(
        &signing_policy,
        4_000,
        &mut reused_signer,
    ))
    .expect("signed multi-issuer batch should be reused");

    assert_eq!(reused.batch_id, Some(batch_id));
    assert!(reused.reused_signed);
    assert_eq!(reused_signer.public_key_calls, 0);
    assert_eq!(reused_signer.sign_calls, 0);
}

#[test]
fn chain_key_batch_signing_failure_marks_same_batch_retryable() {
    let signing_policy = signing_policy();
    let issuer = p(52);
    AuthStateOps::upsert_root_issuer_policy(policy(issuer));
    AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));
    let prepared = prepare_due_chain_key_root_delegation_batch(input(&signing_policy))
        .expect("prepare should build a batch");
    let batch = AuthStateOps::chain_key_root_delegation_batch(prepared.batch_id.unwrap())
        .expect("prepared batch should be stored");
    let mut signer = MockSigner::valid_for(&batch.header);
    signer.public_key[0] ^= 1;

    let err = block_on(sign_next_chain_key_root_delegation_batch(
        &signing_policy,
        2_000,
        &mut signer,
    ))
    .expect_err("public-key mismatch should fail signing");

    assert!(err.to_string().contains("signing failed"));
    assert_eq!(signer.public_key_calls, 1);
    assert_eq!(signer.sign_calls, 0);
    let stored = AuthStateOps::chain_key_root_delegation_batch(prepared.batch_id.unwrap())
        .expect("failed batch should remain stored");
    assert_eq!(
        stored.status,
        ChainKeyRootDelegationBatchStatus::FailedRetryable
    );
    let retry_after_ns = stored.retry_after_ns.expect("failed batch retry time");
    assert!(retry_after_ns >= 2_000);
    assert!(retry_after_ns < stored.header.expires_at_ns);
    assert!(stored.signature.is_none());

    let mut blocked_signer = MockSigner::valid_for(&stored.header);
    let blocked = block_on(sign_next_chain_key_root_delegation_batch(
        &signing_policy,
        3_000,
        &mut blocked_signer,
    ))
    .expect("retry delay should skip signing");

    assert_eq!(blocked.batch_id, None);
    assert_eq!(blocked_signer.public_key_calls, 0);
    assert_eq!(blocked_signer.sign_calls, 0);

    let mut retry_signer = MockSigner::valid_for(&stored.header);
    let retried = block_on(sign_next_chain_key_root_delegation_batch(
        &signing_policy,
        retry_after_ns,
        &mut retry_signer,
    ))
    .expect("retry delay expiry should allow the same batch to sign");

    assert_eq!(retried.batch_id, prepared.batch_id);
    assert!(retried.signed);
    assert_eq!(retry_signer.public_key_calls, 1);
    assert_eq!(retry_signer.sign_calls, 1);
    let retried_stored = AuthStateOps::chain_key_root_delegation_batch(
        prepared.batch_id.expect("prepared batch id"),
    )
    .expect("retried batch should remain stored");
    assert_eq!(
        retried_stored.status,
        ChainKeyRootDelegationBatchStatus::Signed
    );
    assert_eq!(retried_stored.retry_after_ns, None);
    assert_eq!(retried_stored.failure, None);
    assert!(retried_stored.signature.is_some());
}

#[test]
fn chain_key_batch_duplicate_signing_tick_observes_in_flight_without_management_calls() {
    let signing_policy = signing_policy();
    let issuer = p(58);
    AuthStateOps::upsert_root_issuer_policy(policy(issuer));
    AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));
    let prepared = prepare_due_chain_key_root_delegation_batch(input(&signing_policy))
        .expect("prepare should build a batch");
    let batch_id = prepared.batch_id.expect("prepare should return a batch id");
    let mut batch = AuthStateOps::chain_key_root_delegation_batch(batch_id)
        .expect("prepared batch should be stored");
    batch.status = ChainKeyRootDelegationBatchStatus::Signing;
    AuthStateOps::upsert_chain_key_root_delegation_batch(batch);
    let mut signer = DynamicMockSigner {
        public_key_calls: 0,
        sign_calls: 0,
    };

    let result = block_on(sign_next_chain_key_root_delegation_batch(
        &signing_policy,
        2_000,
        &mut signer,
    ))
    .expect("duplicate signing tick should be a no-op");

    assert_eq!(result.batch_id, Some(batch_id));
    assert!(result.signing_in_flight);
    assert!(!result.signed);
    assert_eq!(signer.public_key_calls, 0);
    assert_eq!(signer.sign_calls, 0);
    let stored = AuthStateOps::chain_key_root_delegation_batch(batch_id)
        .expect("in-flight batch should remain stored");
    assert_eq!(stored.status, ChainKeyRootDelegationBatchStatus::Signing);
    assert!(stored.signature.is_none());
}

#[test]
fn chain_key_batch_discards_signature_returning_after_batch_became_stale() {
    let signing_policy = signing_policy();
    let issuer = p(66);
    AuthStateOps::upsert_root_issuer_policy(policy(issuer));
    AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));
    let prepared = prepare_due_chain_key_root_delegation_batch(input(&signing_policy))
        .expect("prepare should build a batch");
    let batch_id = prepared.batch_id.expect("prepare should return a batch id");
    let mut signer = StaleDuringSignSigner {
        batch_id,
        public_key_calls: 0,
        sign_calls: 0,
    };

    let result = block_on(sign_chain_key_root_delegation_batch(
        &signing_policy,
        batch_id,
        2_000,
        &mut signer,
    ))
    .expect("stale callback should be discarded without failing sweep");

    assert_eq!(result.batch_id, Some(batch_id));
    assert!(!result.signed);
    assert!(!result.reused_signed);
    assert!(!result.signing_in_flight);
    assert_eq!(signer.public_key_calls, 1);
    assert_eq!(signer.sign_calls, 1);
    let stored = AuthStateOps::chain_key_root_delegation_batch(batch_id)
        .expect("stale batch should remain until expiry pruning");
    assert_eq!(
        stored.status,
        ChainKeyRootDelegationBatchStatus::FailedRetryable
    );
    assert!(stored.signature.is_none());
    assert_eq!(
        stored.failure.as_deref(),
        Some("stale registry epoch or hash")
    );
}

#[test]
fn chain_key_batch_registry_change_discards_stale_preinstall_batch() {
    let signing_policy = signing_policy();
    let issuer = p(59);
    AuthStateOps::upsert_root_issuer_policy(policy(issuer));
    AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));
    let prepared = prepare_due_chain_key_root_delegation_batch(input(&signing_policy))
        .expect("prepare should build the original batch");
    let stale_batch_id = prepared.batch_id.expect("original batch id");
    let mut stale_batch = AuthStateOps::chain_key_root_delegation_batch(stale_batch_id)
        .expect("original batch should be stored");
    stale_batch.status = ChainKeyRootDelegationBatchStatus::Signing;
    AuthStateOps::upsert_chain_key_root_delegation_batch(stale_batch);

    let mut changed_registry = input(&signing_policy);
    changed_registry.registry_epoch = 12;
    changed_registry.registry_hash = [33; 32];
    let refreshed = prepare_due_chain_key_root_delegation_batch(changed_registry)
        .expect("registry change should prepare a fresh batch");
    let refreshed_batch_id = refreshed.batch_id.expect("fresh batch id");

    assert_ne!(refreshed_batch_id, stale_batch_id);
    assert!(!refreshed.reused_in_flight);
    let stale_batch = AuthStateOps::chain_key_root_delegation_batch(stale_batch_id)
        .expect("stale batch should remain until expiry pruning");
    assert_eq!(
        stale_batch.status,
        ChainKeyRootDelegationBatchStatus::FailedRetryable
    );
    assert_eq!(
        stale_batch.retry_after_ns,
        Some(stale_batch.header.expires_at_ns)
    );
    assert_eq!(
        stale_batch.failure.as_deref(),
        Some("stale registry epoch or hash")
    );
    let refreshed_batch = AuthStateOps::chain_key_root_delegation_batch(refreshed_batch_id)
        .expect("fresh registry batch should be stored");
    assert_eq!(refreshed_batch.header.registry_epoch, 12);
    assert_eq!(refreshed_batch.header.registry_hash, [33; 32]);

    let mut signer = MockSigner::valid_for(&refreshed_batch.header);
    let signing_result = block_on(sign_next_chain_key_root_delegation_batch(
        &signing_policy,
        2_000,
        &mut signer,
    ))
    .expect("fresh registry batch should sign");

    assert_eq!(signing_result.batch_id, Some(refreshed_batch_id));
    assert!(signing_result.signed);
    assert_eq!(signer.sign_calls, 1);
}

#[test]
fn chain_key_batch_expired_preinstall_batch_is_pruned_before_signing() {
    let signing_policy = signing_policy();
    let issuer = p(63);
    AuthStateOps::upsert_root_issuer_policy(policy(issuer));
    AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));
    let prepared = prepare_due_chain_key_root_delegation_batch(input(&signing_policy))
        .expect("prepare should build a batch");
    let batch_id = prepared.batch_id.expect("prepared batch id");
    let batch = AuthStateOps::chain_key_root_delegation_batch(batch_id)
        .expect("prepared batch should be stored");
    let mut signer = MockSigner::valid_for(&batch.header);

    let result = block_on(sign_next_chain_key_root_delegation_batch(
        &signing_policy,
        batch.header.expires_at_ns,
        &mut signer,
    ))
    .expect("expired batch should be pruned without signing");

    assert_eq!(result.batch_id, None);
    assert_eq!(signer.public_key_calls, 0);
    assert_eq!(signer.sign_calls, 0);
    assert!(
        AuthStateOps::chain_key_root_delegation_batch(batch_id).is_none(),
        "expired pre-install batch must not remain signable"
    );
}

#[test]
fn chain_key_batch_install_plan_materializes_signed_proof_and_records_success() {
    let signing_policy = signing_policy();
    let issuer = p(53);
    AuthStateOps::upsert_root_issuer_policy(policy(issuer));
    AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));
    let mut batch = build_chain_key_root_delegation_batch(
        input(&signing_policy),
        &[DueChainKeyTemplate {
            template: template(issuer, 60_000_000_000),
        }],
        10,
    )
    .expect("batch should build");
    batch.status = ChainKeyRootDelegationBatchStatus::Signed;
    batch.signature = Some(sign_header(&batch.header));
    batch.signed_at_ns = Some(2_000);
    let batch_id = batch.batch_id;
    AuthStateOps::upsert_chain_key_root_delegation_batch(batch);

    let plan = start_chain_key_root_delegation_batch_install(batch_id, 3_000)
        .expect("install planning should succeed")
        .expect("signed batch should produce an install plan");

    assert_eq!(plan.batch_id, batch_id);
    assert_eq!(plan.proofs.len(), 1);
    let stored = AuthStateOps::chain_key_root_delegation_batch(batch_id)
        .expect("installing batch should remain stored");
    assert_eq!(stored.status, ChainKeyRootDelegationBatchStatus::Installing);
    assert_eq!(stored.install_started_at_ns, Some(3_000));

    let proof = &plan.proofs[0];
    assert_eq!(proof.issuer_pid, issuer);
    verify_chain_key_batch_root_proof(
        crate::ops::auth::delegated::chain_key::VerifyChainKeyBatchRootProofInput {
            cert: &proof.proof.cert,
            root_proof: &proof.proof.root_proof,
            policy: &verifier_policy(&signing_policy),
            now_ns: 3_000,
        },
        |input: ChainKeySignatureVerificationInput<'_>| verify_chain_key_ecdsa_signature(input),
    )
    .expect("materialized install proof should verify");

    assert!(record_chain_key_root_delegation_install_success(
        batch_id,
        proof.issuer_pid,
        proof.cert_hash,
        4_000,
    ));
    let installed = AuthStateOps::chain_key_root_delegation_batch(batch_id)
        .expect("installed batch should remain stored");
    assert_eq!(
        installed.status,
        ChainKeyRootDelegationBatchStatus::Installed
    );
    assert_eq!(installed.installed_at_ns, Some(4_000));
    assert_eq!(installed.issuers[0].installed_at_ns, Some(4_000));

    let state = AuthStateOps::root_issuer_renewal_state(issuer)
        .expect("issuer renewal state should be updated");
    assert_eq!(state.last_installed_cert_hash, Some(proof.cert_hash));
    assert_eq!(
        state.last_installed_expires_at_ns,
        Some(proof.proof.cert.expires_at_ns)
    );
    assert_eq!(state.last_outcome, RootIssuerRenewalOutcome::Installed);
    assert_eq!(state.consecutive_failures, 0);
    assert_eq!(
        start_chain_key_root_delegation_batch_install(batch_id, 5_000)
            .expect("installed batch should be ignored"),
        None
    );
}

#[test]
fn chain_key_batch_partial_install_failure_retries_only_remaining_issuer() {
    let signing_policy = signing_policy();
    let issuer_a = p(64);
    let issuer_b = p(65);
    AuthStateOps::upsert_root_issuer_policy(policy(issuer_a));
    AuthStateOps::upsert_root_issuer_policy(policy(issuer_b));
    AuthStateOps::upsert_root_issuer_renewal_template(template(issuer_a, 60_000_000_000));
    AuthStateOps::upsert_root_issuer_renewal_template(template(issuer_b, 60_000_000_000));
    let mut batch = build_chain_key_root_delegation_batch(
        input(&signing_policy),
        &[
            DueChainKeyTemplate {
                template: template(issuer_a, 60_000_000_000),
            },
            DueChainKeyTemplate {
                template: template(issuer_b, 60_000_000_000),
            },
        ],
        10,
    )
    .expect("batch should build");
    batch.status = ChainKeyRootDelegationBatchStatus::Signed;
    batch.signature = Some(sign_header(&batch.header));
    batch.signed_at_ns = Some(2_000);
    let batch_id = batch.batch_id;
    AuthStateOps::upsert_chain_key_root_delegation_batch(batch);

    let plan = start_chain_key_root_delegation_batch_install(batch_id, 3_000)
        .expect("install planning should succeed")
        .expect("signed batch should produce an install plan");
    assert_eq!(plan.proofs.len(), 2);
    let installed = plan.proofs[0].clone();
    let failed = plan.proofs[1].clone();

    assert!(record_chain_key_root_delegation_install_success(
        batch_id,
        installed.issuer_pid,
        installed.cert_hash,
        4_000,
    ));
    assert!(record_chain_key_root_delegation_install_failure(
        batch_id,
        failed.issuer_pid,
        failed.cert_hash,
        RootDelegationProofInstallOutcome::CallFailed,
    ));

    let partially_installed = AuthStateOps::chain_key_root_delegation_batch(batch_id)
        .expect("partially installed batch should remain stored");
    assert_eq!(
        partially_installed.status,
        ChainKeyRootDelegationBatchStatus::Installing
    );
    assert_eq!(partially_installed.failure, Some("CallFailed".to_string()));
    assert!(
        partially_installed
            .issuers
            .iter()
            .any(|issuer| issuer.issuer_pid == installed.issuer_pid
                && issuer.installed_at_ns == Some(4_000))
    );
    assert!(
        partially_installed
            .issuers
            .iter()
            .any(|issuer| issuer.issuer_pid == failed.issuer_pid
                && issuer.installed_at_ns.is_none()
                && issuer.last_failure.as_deref() == Some("CallFailed"))
    );

    let retry_plan = start_chain_key_root_delegation_batch_install(batch_id, 5_000)
        .expect("partial retry planning should succeed")
        .expect("failed issuer should remain installable");

    assert_eq!(retry_plan.proofs.len(), 1);
    assert_eq!(retry_plan.proofs[0].issuer_pid, failed.issuer_pid);
    assert!(record_chain_key_root_delegation_install_success(
        batch_id,
        failed.issuer_pid,
        failed.cert_hash,
        6_000,
    ));

    let completed = AuthStateOps::chain_key_root_delegation_batch(batch_id)
        .expect("completed batch should remain stored");
    assert_eq!(
        completed.status,
        ChainKeyRootDelegationBatchStatus::Installed
    );
    assert_eq!(completed.installed_at_ns, Some(6_000));
    assert_eq!(completed.failure, None);
    assert!(
        completed
            .issuers
            .iter()
            .all(|issuer| issuer.installed_at_ns.is_some() && issuer.last_failure.is_none())
    );
}

#[test]
fn chain_key_batch_ignores_stale_install_failure_after_success() {
    let signing_policy = signing_policy();
    let issuer = p(66);
    AuthStateOps::upsert_root_issuer_policy(policy(issuer));
    AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));
    let mut batch = build_chain_key_root_delegation_batch(
        input(&signing_policy),
        &[DueChainKeyTemplate {
            template: template(issuer, 60_000_000_000),
        }],
        10,
    )
    .expect("batch should build");
    batch.status = ChainKeyRootDelegationBatchStatus::Signed;
    batch.signature = Some(sign_header(&batch.header));
    batch.signed_at_ns = Some(2_000);
    let batch_id = batch.batch_id;
    AuthStateOps::upsert_chain_key_root_delegation_batch(batch);

    let plan = start_chain_key_root_delegation_batch_install(batch_id, 3_000)
        .expect("install planning should succeed")
        .expect("signed batch should produce an install plan");
    let proof = &plan.proofs[0];

    assert!(record_chain_key_root_delegation_install_success(
        batch_id,
        proof.issuer_pid,
        proof.cert_hash,
        4_000,
    ));
    assert!(!record_chain_key_root_delegation_install_failure(
        batch_id,
        proof.issuer_pid,
        proof.cert_hash,
        RootDelegationProofInstallOutcome::CallFailed,
    ));

    let installed = AuthStateOps::chain_key_root_delegation_batch(batch_id)
        .expect("installed batch should remain stored");
    assert_eq!(
        installed.status,
        ChainKeyRootDelegationBatchStatus::Installed
    );
    assert_eq!(installed.installed_at_ns, Some(4_000));
    assert_eq!(installed.failure, None);
    assert_eq!(installed.issuers[0].installed_at_ns, Some(4_000));
    assert_eq!(installed.issuers[0].last_failure, None);
}

#[test]
fn chain_key_lazy_repair_get_or_create_signs_once_then_reuses_cached_proof() {
    let signing_policy = signing_policy();
    let issuer = p(54);
    AuthStateOps::upsert_root_issuer_policy(policy(issuer));
    AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));
    let mut signer = DynamicMockSigner {
        public_key_calls: 0,
        sign_calls: 0,
    };

    let proof = block_on(get_or_create_chain_key_delegation_proof_for_issuer(
        input(&signing_policy),
        issuer,
        &mut signer,
    ))
    .expect("lazy repair should sign")
    .expect("lazy repair should return a proof");

    assert_eq!(proof.issuer_pid, issuer);
    assert_eq!(signer.public_key_calls, 1);
    assert_eq!(signer.sign_calls, 1);
    verify_chain_key_batch_root_proof(
        crate::ops::auth::delegated::chain_key::VerifyChainKeyBatchRootProofInput {
            cert: &proof.proof.cert,
            root_proof: &proof.proof.root_proof,
            policy: &verifier_policy(&signing_policy),
            now_ns: 1_000,
        },
        |input: ChainKeySignatureVerificationInput<'_>| verify_chain_key_ecdsa_signature(input),
    )
    .expect("lazy repair proof should verify");

    let mut cached_signer = DynamicMockSigner {
        public_key_calls: 0,
        sign_calls: 0,
    };
    let cached = block_on(get_or_create_chain_key_delegation_proof_for_issuer(
        input(&signing_policy),
        issuer,
        &mut cached_signer,
    ))
    .expect("cached lazy repair should succeed")
    .expect("cached lazy repair should return a proof");

    assert_eq!(cached.cert_hash, proof.cert_hash);
    assert_eq!(cached_signer.public_key_calls, 0);
    assert_eq!(cached_signer.sign_calls, 0);
}

#[test]
fn chain_key_lazy_repair_reuses_in_flight_batch_without_extra_signing() {
    let signing_policy = signing_policy();
    let issuer = p(55);
    AuthStateOps::upsert_root_issuer_policy(policy(issuer));
    AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));

    let prepared = prepare_due_chain_key_root_delegation_batch(input(&signing_policy))
        .expect("prepare should build a batch");
    let batch_id = prepared.batch_id.expect("prepare should return a batch id");
    let mut batch = AuthStateOps::chain_key_root_delegation_batch(batch_id)
        .expect("prepared batch should be stored");
    batch.status = ChainKeyRootDelegationBatchStatus::Signing;
    AuthStateOps::upsert_chain_key_root_delegation_batch(batch);

    for _ in 0..8 {
        let mut signer = DynamicMockSigner {
            public_key_calls: 0,
            sign_calls: 0,
        };
        let proof = block_on(get_or_create_chain_key_delegation_proof_for_issuer(
            input(&signing_policy),
            issuer,
            &mut signer,
        ))
        .expect("in-flight lazy repair should be retryable later");

        assert_eq!(proof, None);
        assert_eq!(signer.public_key_calls, 0);
        assert_eq!(signer.sign_calls, 0);
    }

    assert_eq!(AuthStateOps::chain_key_root_delegation_batches().len(), 1);
    let stored = AuthStateOps::chain_key_root_delegation_batch(batch_id)
        .expect("in-flight batch should remain stored");
    assert_eq!(stored.status, ChainKeyRootDelegationBatchStatus::Signing);
    assert!(stored.signature.is_none());
}

#[test]
fn chain_key_lazy_repair_respects_retry_after_before_resigning() {
    let signing_policy = signing_policy();
    let issuer = p(56);
    AuthStateOps::upsert_root_issuer_policy(policy(issuer));
    AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));

    let prepared = prepare_due_chain_key_root_delegation_batch(input(&signing_policy))
        .expect("prepare should build a batch");
    let batch_id = prepared.batch_id.expect("prepare should return a batch id");
    let retry_after_ns = 60_000;
    let mut batch = AuthStateOps::chain_key_root_delegation_batch(batch_id)
        .expect("prepared batch should be stored");
    batch.status = ChainKeyRootDelegationBatchStatus::FailedRetryable;
    batch.retry_after_ns = Some(retry_after_ns);
    batch.failure = Some("previous signing attempt failed".to_string());
    AuthStateOps::upsert_chain_key_root_delegation_batch(batch);

    let mut early_input = input(&signing_policy);
    early_input.now_ns = retry_after_ns - 1;
    let mut early_signer = DynamicMockSigner {
        public_key_calls: 0,
        sign_calls: 0,
    };
    let early = block_on(get_or_create_chain_key_delegation_proof_for_issuer(
        early_input,
        issuer,
        &mut early_signer,
    ))
    .expect("early lazy repair should remain retryable");

    assert_eq!(early, None);
    assert_eq!(early_signer.public_key_calls, 0);
    assert_eq!(early_signer.sign_calls, 0);

    let mut retry_input = input(&signing_policy);
    retry_input.now_ns = retry_after_ns;
    let mut retry_signer = DynamicMockSigner {
        public_key_calls: 0,
        sign_calls: 0,
    };
    let retried = block_on(get_or_create_chain_key_delegation_proof_for_issuer(
        retry_input,
        issuer,
        &mut retry_signer,
    ))
    .expect("retry-window lazy repair should sign")
    .expect("retry-window lazy repair should return a proof");

    assert_eq!(retried.issuer_pid, issuer);
    assert_eq!(retry_signer.public_key_calls, 1);
    assert_eq!(retry_signer.sign_calls, 1);
    let stored = AuthStateOps::chain_key_root_delegation_batch(batch_id)
        .expect("retried batch should remain stored");
    assert_eq!(stored.status, ChainKeyRootDelegationBatchStatus::Signed);
    assert_eq!(stored.retry_after_ns, None);
    assert_eq!(stored.failure, None);
}

#[test]
fn chain_key_template_due_respects_refresh_and_template_fingerprint() {
    let issuer = p(60);
    let template = template(issuer, 60_000_000_000);
    let fingerprint = renewal_template_fingerprint(&template);
    let state = RootIssuerRenewalState {
        issuer_pid: issuer,
        template_fingerprint: fingerprint,
        last_installed_cert_hash: Some([1; 32]),
        last_installed_expires_at_ns: Some(200),
        last_installed_refresh_after_ns: Some(100),
        active_attempt_id: None,
        last_outcome: RootIssuerRenewalOutcome::Installed,
        consecutive_failures: 0,
        next_attempt_after_ns: 0,
        updated_at_ns: 10,
    };

    assert!(!chain_key_template_due(99, fingerprint, Some(&state)));
    assert!(chain_key_template_due(100, fingerprint, Some(&state)));
    assert!(chain_key_template_due(99, [9; 32], Some(&state)));

    let mut delayed = state;
    delayed.next_attempt_after_ns = 150;
    assert!(!chain_key_template_due(100, fingerprint, Some(&delayed)));
}

#[test]
fn merkle_witnesses_round_trip_for_odd_leaf_count() {
    let leaves = [[1; 32], [2; 32], [3; 32]];
    let (root, witnesses) = merkle_root_and_witnesses(&leaves).expect("tree should build");

    assert_eq!(witnesses.len(), 3);
    for (leaf, witness) in leaves.into_iter().zip(witnesses) {
        let witness_root = witness.steps.iter().fold(leaf, |current, step| match step {
            ChainKeyBatchWitnessStepV1::LeftSibling(sibling) => {
                chain_key_batch_node_hash(*sibling, current)
            }
            ChainKeyBatchWitnessStepV1::RightSibling(sibling) => {
                chain_key_batch_node_hash(current, *sibling)
            }
        });
        assert_eq!(witness_root, root);
    }
}
