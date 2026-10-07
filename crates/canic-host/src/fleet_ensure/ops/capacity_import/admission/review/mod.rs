//! Resolve current artifact and physical authority before spending initial observation allowances.
//!
//! Workflow owns the Fleet lock and reserve/read/retain sequence. This owner constructs records.

mod provenance;

use crate::{
    fleet_ensure::{
        dto::capacity_import::CapacityImportReviewRequest,
        model::{
            DesiredFleet, FleetEnsureStateRecord,
            capacity_import::{
                CapacityImportAuthority, CapacityImportPlanRecord, CapacityImportRootBudget,
                CapacityImportSourceBinding, CapacityImportSourceRecord,
                admission::CapacityImportAdmissionRecord,
                funding::CapacityImportFundingCreditRecord, survey::CapacityImportSampleRecord,
            },
        },
        ops::{
            EnsurePaths,
            capacity_import::{
                admission::{
                    declarations,
                    observer::{inventory, management},
                },
                funding,
                journal::CapacityImportJournalError,
                prepare_review,
                transport::CapacityImportTransport,
                validate_destination_authority, with_admission, with_funding,
            },
            certified_custody,
        },
        policy::capacity_import::{CapacityImportPolicyError, admit_handoffs, select_destination},
        view::capacity_import::{
            CapacityImportDestinationView, CapacityImportFundingBaselineView,
            CapacityImportFundingSampleView, CapacityImportOwnershipView, CapacityImportRootView,
            CapacityImportSourceView,
        },
    },
    icp::IcpCli,
};

use candid::Principal;
use canic_core::{
    cdk::utils::hash::hex_bytes,
    dto::{fleet_registry::FleetRegistry, pool_import::PoolImportContext},
    ids::{CanonicalNetworkId, MAX_FLEET_CAPACITY_IMPORT_SOURCES},
};
use sha2_host::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

use ic_host_fs::durable::read_regular_bytes;

/// Frozen free-query admission, retained until the initial management samples are committed.
pub(in crate::fleet_ensure) struct ReviewSurvey {
    transport: CapacityImportTransport,
    authority: CapacityImportAuthority,
    admission: CapacityImportAdmissionRecord,
    registry: FleetRegistry,
    context: PoolImportContext,
    inventory: inventory::Inventory,
    request_sha256: [u8; 32],
    original_request_sha256: [u8; 32],
    funding_credits: BTreeMap<Principal, u128>,
}

impl ReviewSurvey {
    /// Validate exact source selection and current infrastructure without management updates.
    pub(crate) async fn inspect(
        paths: &EnsurePaths,
        request: &CapacityImportReviewRequest,
        desired: &DesiredFleet,
        state: &FleetEnsureStateRecord,
        icp: &IcpCli,
    ) -> Result<Self, CapacityImportJournalError> {
        let bytes = read_regular_bytes(&paths.workspace.join(&request.declarations), 256 * 1024)?;
        let text =
            String::from_utf8(bytes).map_err(|_| CapacityImportJournalError::DispositionInvalid)?;
        let declarations = declarations::parse(&text)?;
        let bindings = source_bindings(request, &declarations)?;
        let transport = CapacityImportTransport::from_icp(icp)?;
        let operator = transport
            .agent
            .get_principal()
            .map_err(|_| CapacityImportJournalError::ReaderMismatch)?;
        let root_key = transport.agent.read_root_key();
        let key_hash: [u8; 32] = Sha256::digest(&root_key).into();
        let registry = state
            .active_registry
            .as_ref()
            .ok_or(CapacityImportJournalError::InfrastructureRequired)?;
        if declarations.operator != operator.to_text()
            || desired.operator != operator.to_text()
            || declarations::hash(&declarations.network_root_key_sha256)? != key_hash
            || CanonicalNetworkId::from_der_root_trust_anchor(&root_key).ok()
                != Some(registry.authority.binding.fleet.fleet.canonical_network_id)
        {
            return Err(CapacityImportJournalError::ReaderMismatch);
        }
        let subnet = bindings[0].subnet;
        if let Some(binding) = bindings.iter().find(|binding| binding.subnet != subnet) {
            return Err(CapacityImportPolicyError::WrongSubnet {
                canister: binding.canister_id,
            }
            .into());
        }
        let roots = registry
            .fleet_subnet_roots
            .iter()
            .map(|entry| CapacityImportRootView {
                root: entry.fleet_subnet_root,
                subnet: entry.placement_subnet,
            })
            .collect::<Vec<_>>();
        let root = select_destination(&roots, subnet, request.root)?.root;
        let funding_credits = funding::requested(request, root)?;
        let infrastructure = provenance::inspect(paths, desired, state, &transport.agent).await?;
        if inventory::registry(&transport.agent, registry.authority.binding.coordinator).await?
            != *registry
        {
            return Err(CapacityImportJournalError::InfrastructureChanged);
        }
        let context = transport.root_context(root).await?;
        super::validate_budget(request, context.maximum_call_debit_cycles)?;
        if context.active_import.is_some() {
            return Err(CapacityImportJournalError::Conflict);
        }
        let admission = CapacityImportAdmissionRecord {
            infrastructure,
            registry_candid_hex: hex_bytes(
                candid::encode_one(registry)
                    .map_err(|_| CapacityImportJournalError::InventoryInvalid)?,
            ),
            declarations_sha256: Sha256::digest(text.as_bytes()).into(),
            declarations_toml: text,
        };
        let inventory = inventory::observe(&transport.agent, &admission, root, registry).await?;
        verify_sources(&transport.agent, &inventory, operator, root, &bindings).await?;
        let authority = CapacityImportAuthority {
            fleet: registry.authority.binding.fleet.clone(),
            network_root_key_sha256: key_hash,
            operator,
            coordinator: registry.authority.binding.coordinator,
            root,
            subnet,
            root_authority_sha256: context.root_authority_sha256,
            import_sequence: context.next_sequence,
            recovery_controllers: registry.authority.binding.recovery_controllers.clone(),
        };
        crate::fleet_ensure::ops::capacity_import::destination::validate_authority(
            &authority, &context, registry,
        )?;
        let request_sha256 = request_digest(paths, request, &authority, &admission)?;
        let mut original_request = request.clone();
        original_request.funding_credits.clear();
        let original_request_sha256 =
            request_digest(paths, &original_request, &authority, &admission)?;
        Ok(Self {
            transport,
            authority,
            admission,
            registry: registry.clone(),
            context,
            inventory,
            request_sha256,
            original_request_sha256,
            funding_credits,
        })
    }

    /// A setup receipt admits only the exact still-held source set for its selected Root.
    pub(crate) fn require_bootstrap_sources(
        &self,
        source: &crate::fleet_ensure::model::infrastructure_bootstrap::InfrastructureBootstrapRecord,
        request: &CapacityImportReviewRequest,
    ) -> Result<(), CapacityImportJournalError> {
        let hold = self
            .context
            .bootstrap
            .as_ref()
            .ok_or(CapacityImportJournalError::InfrastructureRequired)?;
        let selected = request.canisters.iter().copied().collect::<BTreeSet<_>>();
        let expected = hold.sources.iter().copied().collect::<BTreeSet<_>>();
        let original = source
            .sources
            .values()
            .map(|source| source.sample.binding.canister_id)
            .chain(
                source
                    .held_sources
                    .values()
                    .map(|source| source.custody.canister),
            )
            .collect::<BTreeSet<_>>();
        let original_review_matches = hold.review_sha256 == source.source_sha256;
        if !original_review_matches
            || hold.operator != source.operator
            || selected != expected
            || !expected.is_subset(&original)
        {
            return Err(CapacityImportJournalError::InfrastructureChanged);
        }
        Ok(())
    }

    pub(crate) const fn root(&self) -> Principal {
        self.authority.root
    }

    pub(crate) const fn request_sha256(&self) -> [u8; 32] {
        self.request_sha256
    }

    pub(crate) const fn original_request_sha256(&self) -> [u8; 32] {
        self.original_request_sha256
    }

    pub(crate) fn observes_funding(&self, canister: Principal) -> bool {
        self.funding_credits.contains_key(&canister)
    }

    /// Preserve original accounting while binding one expressly declared credit to a fresh sample.
    pub(crate) fn recognize_funding(
        &self,
        request: &CapacityImportReviewRequest,
        baseline: CapacityImportFundingBaselineView,
        observed: CapacityImportSampleRecord,
    ) -> Result<CapacityImportFundingSampleView, CapacityImportJournalError> {
        let canister = observed.binding.canister_id;
        let amount = *self
            .funding_credits
            .get(&canister)
            .ok_or(CapacityImportJournalError::Integrity)?;
        let (maximum, minimum) = if canister == self.authority.root {
            (
                request.maximum_root_debit_cycles,
                self.context
                    .binding
                    .funding
                    .root_funding
                    .request_threshold
                    .to_u128(),
            )
        } else {
            (
                request.maximum_source_debit_cycles,
                self.context
                    .binding
                    .limits
                    .canister_pool
                    .canister_cycles
                    .to_u128(),
            )
        };
        funding::recognize(
            baseline,
            observed,
            amount,
            self.authority.root,
            maximum,
            minimum,
        )
    }

    pub(crate) fn canisters(&self, request: &CapacityImportReviewRequest) -> Vec<Principal> {
        let mut ids = request.canisters.clone();
        ids.push(self.authority.root);
        ids
    }

    /// Complete free checks before workflow durably reserves the one status update.
    pub(crate) async fn prepare_sample(
        &self,
        canister: Principal,
    ) -> Result<management::PreparedManagementObservation, CapacityImportJournalError> {
        let declarations = declarations::parse(&self.admission.declarations_toml)?;
        let held = declarations
            .canisters
            .iter()
            .map(declarations::CapacityImportDeclaration::binding)
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .any(|source| {
                source.canister_id == canister && source.controllers.contains(&self.authority.root)
            });
        if held {
            management::prepare_root_owned(&self.transport.agent, self.authority.root, canister)
                .await
        } else {
            management::prepare(&self.transport.agent, canister).await
        }
    }

    /// Construct the exact original review and recheck free admission after all paid samples.
    pub(crate) async fn finish(
        self,
        request: &CapacityImportReviewRequest,
        samples: &[CapacityImportSampleRecord],
        credits: Vec<CapacityImportFundingCreditRecord>,
    ) -> Result<CapacityImportPlanRecord, CapacityImportJournalError> {
        let minimum_ready = self
            .context
            .binding
            .limits
            .canister_pool
            .canister_cycles
            .to_u128();
        let declarations = declarations::parse(&self.admission.declarations_toml)?;
        let mut sources = Vec::new();
        for declaration in declarations.canisters {
            let binding = declaration.binding()?;
            let sample = samples
                .iter()
                .find(|sample| sample.binding.canister_id == binding.canister_id)
                .ok_or(CapacityImportJournalError::Integrity)?;
            sources.push(CapacityImportSourceRecord {
                binding: sample.binding.clone(),
                disposition: declaration.disposition(self.admission.declarations_sha256),
                observed_cycles: sample.cycles,
                observed_reserved_cycles: sample.reserved_cycles,
                minimum_ready_cycles: minimum_ready,
                maximum_debit_cycles: request.maximum_source_debit_cycles,
            });
        }
        let root = samples
            .iter()
            .find(|sample| sample.binding.canister_id == self.authority.root)
            .ok_or(CapacityImportJournalError::Integrity)?;
        let root_budget = CapacityImportRootBudget {
            observed_cycles: root.cycles,
            observed_reserved_cycles: root.reserved_cycles,
            minimum_retained_cycles: self
                .context
                .binding
                .funding
                .root_funding
                .request_threshold
                .to_u128(),
            maximum_debit_cycles: request.maximum_root_debit_cycles,
            maximum_paid_calls: request.maximum_root_paid_calls,
        };
        let plan = with_admission(
            with_funding(
                prepare_review(self.authority, sources, root_budget)?,
                credits,
            )?,
            self.admission,
        )?;
        validate_destination_authority(&plan, &self.context, &self.registry)?;
        self.transport.verify_destination(&plan).await?;
        let refreshed = inventory::observe(
            &self.transport.agent,
            plan.admission
                .as_ref()
                .ok_or(CapacityImportJournalError::InfrastructureRequired)?,
            plan.authority.root,
            &self.registry,
        )
        .await?;
        if refreshed != self.inventory
            || self.transport.root_context(plan.authority.root).await? != self.context
        {
            return Err(CapacityImportJournalError::InventoryInvalid);
        }
        let destination = CapacityImportDestinationView {
            authority: plan.authority.clone(),
            ready: true,
            draining: false,
            competing_operation: false,
            occupied_capacity: refreshed.occupied,
            maximum_capacity: refreshed.maximum,
            controlled_cycles: root.cycles,
            reserved_cycles: root.reserved_cycles,
            minimum_retained_cycles: root_budget.minimum_retained_cycles,
            assigned_canisters: refreshed.assigned,
        };
        let sources = plan
            .sources
            .iter()
            .map(|source| CapacityImportSourceView {
                binding: source.binding.clone(),
                cycles: source.observed_cycles,
                reserved_cycles: source.observed_reserved_cycles,
                ownership: CapacityImportOwnershipView::Unassigned,
                disposition_evidence_sha256: Some(
                    crate::fleet_ensure::policy::capacity_import::disposition_digest(source),
                ),
            })
            .collect::<Vec<_>>();
        admit_handoffs(&plan, &destination, &sources)?;
        Ok(plan)
    }
}

fn source_bindings(
    request: &CapacityImportReviewRequest,
    declarations: &crate::fleet_ensure::ops::capacity_import::admission::CapacityImportDeclarations,
) -> Result<Vec<CapacityImportSourceBinding>, CapacityImportJournalError> {
    let selected = request.canisters.iter().copied().collect::<BTreeSet<_>>();
    let mut declared = BTreeSet::new();
    let bindings = declarations
        .canisters
        .iter()
        .map(|declaration| {
            let binding = declaration.binding()?;
            if !declared.insert(binding.canister_id)
                || !declaration.no_external_obligations
                || !declaration.no_other_fleet_ownership
                || declaration.evidence.trim().is_empty()
            {
                return Err(CapacityImportJournalError::DispositionInvalid);
            }
            Ok(binding)
        })
        .collect::<Result<Vec<_>, CapacityImportJournalError>>()?;
    if declarations.schema_version != 1
        || selected.is_empty()
        || selected.len() != request.canisters.len()
        || selected.len() > MAX_FLEET_CAPACITY_IMPORT_SOURCES
        || selected != declared
        || request.maximum_source_debit_cycles == 0
        || request.maximum_root_debit_cycles == 0
        || request.maximum_root_paid_calls == 0
    {
        return Err(CapacityImportJournalError::DispositionInvalid);
    }
    Ok(bindings)
}

async fn verify_sources(
    agent: &ic_agent::Agent,
    inventory: &inventory::Inventory,
    operator: Principal,
    root: Principal,
    bindings: &[CapacityImportSourceBinding],
) -> Result<(), CapacityImportJournalError> {
    for binding in bindings {
        if inventory.assigned.contains(&binding.canister_id) {
            return Err(CapacityImportPolicyError::SourceAssigned {
                canister: binding.canister_id,
            }
            .into());
        }
        let observed = certified_custody::observe_one(agent, binding.canister_id)
            .await
            .map_err(|_| CapacityImportJournalError::ObservationUnavailable {
                canister: binding.canister_id,
            })?;
        if (
            observed.subnet,
            observed.controllers,
            observed
                .module_sha256
                .as_deref()
                .map(declarations::hash)
                .transpose()?,
        ) != (
            binding.subnet,
            binding.controllers.clone(),
            binding.module_sha256,
        ) {
            return Err(CapacityImportPolicyError::SourceChanged {
                canister: binding.canister_id,
            }
            .into());
        }
        if !binding.controllers.contains(&operator) && !binding.controllers.contains(&root) {
            return Err(CapacityImportPolicyError::OperatorNotController {
                canister: binding.canister_id,
            }
            .into());
        }
    }
    Ok(())
}

fn request_digest(
    paths: &EnsurePaths,
    request: &CapacityImportReviewRequest,
    authority: &CapacityImportAuthority,
    admission: &CapacityImportAdmissionRecord,
) -> Result<[u8; 32], CapacityImportJournalError> {
    let mut canonical = request.clone();
    canonical.canisters.sort_unstable();
    canonical
        .funding_credits
        .sort_by_key(|credit| credit.canister);
    let mut hash = Sha256::new();
    hash.update(b"canic:capacity-import-initial-survey:v1\0");
    hash.update(serde_json::to_vec(&(&canonical, authority, admission))?);
    // Byte identities, not a new clock value, own retry allowances and successful samples.
    for path in [
        &paths.plan,
        &paths.journal,
        &paths.state,
        &paths.workspace.join(&request.policy),
        &paths.workspace.join(&request.seed),
    ] {
        hash.update(Sha256::digest(read_regular_bytes(path, 8 * 1024 * 1024)?));
    }
    Ok(hash.finalize().into())
}
