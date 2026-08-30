//! Module: config::component_group
//!
//! Responsibility: compile Component Group declarations into one canonical acyclic graph.
//! Does not own: final deployment purpose resolution, placement, persistence, or runtime parentage.
//! Boundary: validated checked-in declarations become bounded occurrence-preserving projections.

mod label;
#[cfg(test)]
mod tests;

use crate::{
    config::schema::{ComponentGroupSpecConfig, ConfigModel},
    ids::{
        ComponentGroupMemberId, ComponentGroupMemberPath, ComponentGroupMemberPathError,
        ComponentGroupSpecId, ComponentSpecId, FleetServiceId,
    },
};
use std::collections::{BTreeMap, BTreeSet};

use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

pub use label::{
    ComponentDeploymentLabel, ComponentDeploymentLabelKey, ComponentDeploymentLabelParseError,
    ComponentDeploymentLabelValue, MAX_COMPONENT_DEPLOYMENT_LABEL_KEY_BYTES,
    MAX_COMPONENT_DEPLOYMENT_LABEL_VALUE_BYTES, MAX_COMPONENT_DEPLOYMENT_LABELS,
};

const COMPONENT_GROUP_GRAPH_DOMAIN: &[u8] = b"canic/component-group-graph/v1";
const COMPONENT_GROUP_GRAPH_SCHEMA_VERSION: u32 = 1;

/// Maximum Component Group declarations in one App.
pub const MAX_COMPONENT_GROUP_SPECS: usize = 256;
/// Maximum direct Component or included-group members in one declaration.
pub const MAX_COMPONENT_GROUP_MEMBERS: usize = 256;
/// Maximum direct members across the complete declaration graph.
pub const MAX_COMPONENT_GROUP_DECLARED_MEMBERS: usize = 16_384;
/// Maximum included-group edges across the complete declaration graph.
pub const MAX_COMPONENT_GROUP_INCLUSIONS: usize = 4_096;
/// Maximum flattened Component occurrences emitted by any one selected group.
pub const MAX_COMPONENT_GROUP_FLATTENED_MEMBERS: usize = 4_096;
/// Maximum canonical bytes for the complete Component Group declaration graph.
pub const MAX_COMPONENT_GROUP_GRAPH_CANONICAL_BYTES: usize = 2_097_152;

impl ConfigModel {
    /// Compile checked-in Component Group declarations into canonical graph order.
    pub fn compile_component_group_topology(
        &self,
    ) -> Result<ComponentGroupTopology, ComponentGroupTopologyError> {
        ComponentGroupTopology::compile(self)
    }
}

/// Canonical checked-in Component Group declaration graph.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentGroupTopology {
    pub component_groups: Vec<ComponentGroupSpec>,
}

impl ComponentGroupTopology {
    /// Compile bounded declarations and prove every group can flatten acyclically.
    pub fn compile(config: &ConfigModel) -> Result<Self, ComponentGroupTopologyError> {
        if config.component_groups.len() > MAX_COMPONENT_GROUP_SPECS {
            return Err(ComponentGroupTopologyError::GroupBoundExceeded {
                actual: config.component_groups.len(),
                maximum: MAX_COMPONENT_GROUP_SPECS,
            });
        }

        let mut declared_members = 0_usize;
        let mut inclusions = 0_usize;
        let mut component_groups = Vec::with_capacity(config.component_groups.len());
        for (component_group, source) in &config.component_groups {
            let member_count = checked_member_count(component_group, source)?;
            declared_members = declared_members.checked_add(member_count).ok_or(
                ComponentGroupTopologyError::DeclaredMemberBoundExceeded {
                    actual: usize::MAX,
                    maximum: MAX_COMPONENT_GROUP_DECLARED_MEMBERS,
                },
            )?;
            if declared_members > MAX_COMPONENT_GROUP_DECLARED_MEMBERS {
                return Err(ComponentGroupTopologyError::DeclaredMemberBoundExceeded {
                    actual: declared_members,
                    maximum: MAX_COMPONENT_GROUP_DECLARED_MEMBERS,
                });
            }
            inclusions = inclusions.checked_add(source.groups.len()).ok_or(
                ComponentGroupTopologyError::InclusionBoundExceeded {
                    actual: usize::MAX,
                    maximum: MAX_COMPONENT_GROUP_INCLUSIONS,
                },
            )?;
            if inclusions > MAX_COMPONENT_GROUP_INCLUSIONS {
                return Err(ComponentGroupTopologyError::InclusionBoundExceeded {
                    actual: inclusions,
                    maximum: MAX_COMPONENT_GROUP_INCLUSIONS,
                });
            }
            component_groups.push(compile_group(config, component_group, source)?);
        }

        let topology = Self { component_groups };
        topology.canonical_bytes()?;
        Ok(topology)
    }

    /// Return one exact canonical declaration.
    #[must_use]
    pub fn get(&self, component_group: &ComponentGroupSpecId) -> Option<&ComponentGroupSpec> {
        self.component_groups
            .binary_search_by(|candidate| candidate.component_group.cmp(component_group))
            .ok()
            .map(|index| &self.component_groups[index])
    }

    /// Flatten one selected declaration occurrence-by-occurrence without Spec deduplication.
    pub fn flatten(
        &self,
        component_group: &ComponentGroupSpecId,
    ) -> Result<FlattenedComponentGroup, ComponentGroupTopologyError> {
        self.validate_canonical_projection()?;
        self.flatten_canonical(component_group)
    }

    fn flatten_canonical(
        &self,
        component_group: &ComponentGroupSpecId,
    ) -> Result<FlattenedComponentGroup, ComponentGroupTopologyError> {
        if self.get(component_group).is_none() {
            return Err(ComponentGroupTopologyError::UnknownGroup {
                component_group: component_group.clone(),
            });
        }

        let mut components = Vec::new();
        let mut member_path = Vec::new();
        let mut active_groups = Vec::new();
        let mut service_purposes = Vec::new();
        let mut labels = BTreeMap::new();
        self.flatten_into(
            component_group,
            &mut member_path,
            &mut active_groups,
            &mut service_purposes,
            &mut labels,
            &mut components,
        )?;
        Ok(FlattenedComponentGroup {
            component_group: component_group.clone(),
            components,
        })
    }

    /// Return the exact canonical graph bytes used by later semantic digests.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, ComponentGroupTopologyError> {
        self.validate_canonical_projection()?;
        for group in &self.component_groups {
            self.flatten_canonical(&group.component_group)?;
        }
        let mut bytes = Vec::new();
        encode_bytes(&mut bytes, COMPONENT_GROUP_GRAPH_DOMAIN);
        bytes.extend_from_slice(&COMPONENT_GROUP_GRAPH_SCHEMA_VERSION.to_be_bytes());
        encode_u64(&mut bytes, self.component_groups.len());
        for group in &self.component_groups {
            encode_text(&mut bytes, group.component_group.as_str());
            encode_u64(&mut bytes, group.members.len());
            for member in &group.members {
                match member {
                    ComponentGroupMember::Component {
                        member,
                        component_spec,
                        kind,
                        service_purpose,
                        labels,
                    } => {
                        bytes.push(0);
                        encode_text(&mut bytes, member.as_str());
                        encode_text(&mut bytes, component_spec.as_str());
                        encode_leaf_kind(&mut bytes, kind);
                        encode_service_purpose(&mut bytes, service_purpose.as_ref());
                        encode_labels(&mut bytes, labels);
                    }
                    ComponentGroupMember::Group {
                        member,
                        component_group,
                        service_purpose,
                        labels,
                    } => {
                        bytes.push(1);
                        encode_text(&mut bytes, member.as_str());
                        encode_text(&mut bytes, component_group.as_str());
                        encode_service_purpose(&mut bytes, service_purpose.as_ref());
                        encode_labels(&mut bytes, labels);
                    }
                }
            }
        }
        if bytes.len() > MAX_COMPONENT_GROUP_GRAPH_CANONICAL_BYTES {
            return Err(ComponentGroupTopologyError::CanonicalBytesBoundExceeded {
                actual: bytes.len(),
                maximum: MAX_COMPONENT_GROUP_GRAPH_CANONICAL_BYTES,
            });
        }
        Ok(bytes)
    }

    fn validate_canonical_projection(&self) -> Result<(), ComponentGroupTopologyError> {
        if self.component_groups.len() > MAX_COMPONENT_GROUP_SPECS {
            return Err(ComponentGroupTopologyError::GroupBoundExceeded {
                actual: self.component_groups.len(),
                maximum: MAX_COMPONENT_GROUP_SPECS,
            });
        }

        let mut declared_members = 0_usize;
        let mut inclusions = 0_usize;
        let mut previous_group: Option<&ComponentGroupSpecId> = None;
        for group in &self.component_groups {
            if previous_group.is_some_and(|previous| previous >= &group.component_group) {
                return Err(ComponentGroupTopologyError::NonCanonicalGroupOrder {
                    component_group: group.component_group.clone(),
                });
            }
            previous_group = Some(&group.component_group);

            if group.members.is_empty() {
                return Err(ComponentGroupTopologyError::EmptyGroup {
                    component_group: group.component_group.clone(),
                });
            }
            if group.members.len() > MAX_COMPONENT_GROUP_MEMBERS {
                return Err(ComponentGroupTopologyError::MemberBoundExceeded {
                    component_group: group.component_group.clone(),
                    actual: group.members.len(),
                    maximum: MAX_COMPONENT_GROUP_MEMBERS,
                });
            }
            declared_members = declared_members.checked_add(group.members.len()).ok_or(
                ComponentGroupTopologyError::DeclaredMemberBoundExceeded {
                    actual: usize::MAX,
                    maximum: MAX_COMPONENT_GROUP_DECLARED_MEMBERS,
                },
            )?;
            if declared_members > MAX_COMPONENT_GROUP_DECLARED_MEMBERS {
                return Err(ComponentGroupTopologyError::DeclaredMemberBoundExceeded {
                    actual: declared_members,
                    maximum: MAX_COMPONENT_GROUP_DECLARED_MEMBERS,
                });
            }
            let mut previous_member: Option<&ComponentGroupMemberId> = None;
            for member in &group.members {
                let member_id = member.member();
                if previous_member.is_some_and(|previous| previous >= member_id) {
                    return Err(ComponentGroupTopologyError::NonCanonicalMemberOrder {
                        component_group: group.component_group.clone(),
                        member: member_id.clone(),
                    });
                }
                previous_member = Some(member_id);
                validate_member_labels(&group.component_group, member)?;
                if let ComponentGroupMember::Group {
                    component_group: included,
                    ..
                } = member
                {
                    inclusions = inclusions.checked_add(1).ok_or(
                        ComponentGroupTopologyError::InclusionBoundExceeded {
                            actual: usize::MAX,
                            maximum: MAX_COMPONENT_GROUP_INCLUSIONS,
                        },
                    )?;
                    if inclusions > MAX_COMPONENT_GROUP_INCLUSIONS {
                        return Err(ComponentGroupTopologyError::InclusionBoundExceeded {
                            actual: inclusions,
                            maximum: MAX_COMPONENT_GROUP_INCLUSIONS,
                        });
                    }
                    if self.get(included).is_none() {
                        return Err(ComponentGroupTopologyError::UnknownIncludedGroup {
                            component_group: group.component_group.clone(),
                            included: included.clone(),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    fn flatten_into(
        &self,
        component_group: &ComponentGroupSpecId,
        member_path: &mut Vec<ComponentGroupMemberId>,
        active_groups: &mut Vec<ComponentGroupSpecId>,
        service_purposes: &mut Vec<FleetServiceMemberPurpose>,
        effective_labels: &mut BTreeMap<ComponentDeploymentLabelKey, ComponentDeploymentLabelValue>,
        output: &mut Vec<FlattenedComponentGroupMember>,
    ) -> Result<(), ComponentGroupTopologyError> {
        if active_groups.contains(component_group) {
            return Err(ComponentGroupTopologyError::InclusionCycle {
                component_group: component_group.clone(),
            });
        }
        let group =
            self.get(component_group)
                .ok_or_else(|| ComponentGroupTopologyError::UnknownGroup {
                    component_group: component_group.clone(),
                })?;
        active_groups.push(component_group.clone());

        for member in &group.members {
            member_path.push(member.member().clone());
            let output_start = output.len();
            if let Some(purpose) = member.service_purpose() {
                service_purposes.push(purpose);
            }
            let current_path =
                ComponentGroupMemberPath::try_from(member_path.clone()).map_err(|source| {
                    ComponentGroupTopologyError::InvalidMemberPath {
                        component_group: active_groups[0].clone(),
                        source,
                    }
                })?;
            let added_label_keys = extend_effective_labels(
                &active_groups[0],
                &current_path,
                member.labels(),
                effective_labels,
            )?;
            match member {
                ComponentGroupMember::Component {
                    component_spec,
                    kind,
                    ..
                } => {
                    if output.len() >= MAX_COMPONENT_GROUP_FLATTENED_MEMBERS {
                        return Err(ComponentGroupTopologyError::FlattenedMemberBoundExceeded {
                            component_group: active_groups[0].clone(),
                            actual: output.len() + 1,
                            maximum: MAX_COMPONENT_GROUP_FLATTENED_MEMBERS,
                        });
                    }
                    output.push(FlattenedComponentGroupMember {
                        member_path: current_path,
                        component_spec: component_spec.clone(),
                        kind: kind.clone(),
                        service_purpose_assignments: match kind {
                            ComponentGroupLeafKind::Ordinary => Vec::new(),
                            ComponentGroupLeafKind::FleetService { .. } => service_purposes.clone(),
                        },
                        labels: effective_labels
                            .iter()
                            .map(|(key, value)| ComponentDeploymentLabel {
                                key: key.clone(),
                                value: value.clone(),
                            })
                            .collect(),
                    });
                }
                ComponentGroupMember::Group {
                    component_group: included,
                    ..
                } => self.flatten_into(
                    included,
                    member_path,
                    active_groups,
                    service_purposes,
                    effective_labels,
                    output,
                )?,
            }
            if member.service_purpose().is_some()
                && !output[output_start..]
                    .iter()
                    .any(FlattenedComponentGroupMember::is_fleet_service)
            {
                return Err(
                    ComponentGroupTopologyError::InapplicableServicePurposeAssignment {
                        component_group: component_group.clone(),
                        member: member.member().clone(),
                    },
                );
            }
            if member.service_purpose().is_some() {
                service_purposes.pop();
            }
            for key in added_label_keys {
                effective_labels.remove(&key);
            }
            member_path.pop();
        }

        active_groups.pop();
        Ok(())
    }
}

/// One canonical Component Group declaration.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentGroupSpec {
    pub component_group: ComponentGroupSpecId,
    pub members: Vec<ComponentGroupMember>,
}

/// One direct declaration member; included groups remain configuration-only edges.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ComponentGroupMember {
    Component {
        member: ComponentGroupMemberId,
        component_spec: ComponentSpecId,
        kind: ComponentGroupLeafKind,
        service_purpose: Option<FleetServiceMemberPurpose>,
        labels: Vec<ComponentDeploymentLabel>,
    },
    Group {
        member: ComponentGroupMemberId,
        component_group: ComponentGroupSpecId,
        service_purpose: Option<FleetServiceMemberPurpose>,
        labels: Vec<ComponentDeploymentLabel>,
    },
}

impl ComponentGroupMember {
    #[must_use]
    pub const fn member(&self) -> &ComponentGroupMemberId {
        match self {
            Self::Component { member, .. } | Self::Group { member, .. } => member,
        }
    }

    #[must_use]
    pub const fn service_purpose(&self) -> Option<FleetServiceMemberPurpose> {
        match self {
            Self::Component {
                service_purpose, ..
            }
            | Self::Group {
                service_purpose, ..
            } => *service_purpose,
        }
    }

    #[must_use]
    pub fn labels(&self) -> &[ComponentDeploymentLabel] {
        match self {
            Self::Component { labels, .. } | Self::Group { labels, .. } => labels,
        }
    }
}

/// Typed declaration kind for one flattened Component Group leaf.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ComponentGroupLeafKind {
    Ordinary,
    FleetService { service: FleetServiceId },
}

/// Exact semantic purpose assigned to one Fleet-service Component occurrence.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetServiceMemberPurpose {
    #[serde(rename = "authority")]
    Authority,
    #[serde(rename = "replica")]
    Replica,
    #[serde(rename = "pool_member")]
    PoolMember,
}

/// Complete flattened direct-Component occurrences for one selected group.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FlattenedComponentGroup {
    pub component_group: ComponentGroupSpecId,
    pub components: Vec<FlattenedComponentGroupMember>,
}

/// One distinct flattened Component occurrence identified by its full member path.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FlattenedComponentGroupMember {
    pub member_path: ComponentGroupMemberPath,
    pub component_spec: ComponentSpecId,
    pub kind: ComponentGroupLeafKind,
    pub service_purpose_assignments: Vec<FleetServiceMemberPurpose>,
    pub labels: Vec<ComponentDeploymentLabel>,
}

impl FlattenedComponentGroupMember {
    pub(super) const fn is_fleet_service(&self) -> bool {
        matches!(self.kind, ComponentGroupLeafKind::FleetService { .. })
    }
}

/// Typed rejection for invalid Component Group declaration or canonical graph state.
#[derive(Debug, ThisError)]
pub enum ComponentGroupTopologyError {
    #[error("Component Group count {actual} exceeds bound {maximum}")]
    GroupBoundExceeded { actual: usize, maximum: usize },

    #[error("Component Group '{component_group}' must declare at least one member")]
    EmptyGroup {
        component_group: ComponentGroupSpecId,
    },

    #[error("Component Group '{component_group}' member count {actual} exceeds bound {maximum}")]
    MemberBoundExceeded {
        component_group: ComponentGroupSpecId,
        actual: usize,
        maximum: usize,
    },

    #[error("Component Group declared-member count {actual} exceeds bound {maximum}")]
    DeclaredMemberBoundExceeded { actual: usize, maximum: usize },

    #[error("Component Group inclusion count {actual} exceeds bound {maximum}")]
    InclusionBoundExceeded { actual: usize, maximum: usize },

    #[error("Component Group '{component_group}' declares member '{member}' more than once")]
    DuplicateMember {
        component_group: ComponentGroupSpecId,
        member: ComponentGroupMemberId,
    },

    #[error(
        "Component Group '{component_group}' member '{member}' references unknown Component Spec '{component_spec}'"
    )]
    UnknownComponentSpec {
        component_group: ComponentGroupSpecId,
        member: ComponentGroupMemberId,
        component_spec: ComponentSpecId,
    },

    #[error("Component Group '{component_group}' includes unknown Component Group '{included}'")]
    UnknownIncludedGroup {
        component_group: ComponentGroupSpecId,
        included: ComponentGroupSpecId,
    },

    #[error("unknown Component Group '{component_group}'")]
    UnknownGroup {
        component_group: ComponentGroupSpecId,
    },

    #[error("Component Group inclusion cycle involves '{component_group}'")]
    InclusionCycle {
        component_group: ComponentGroupSpecId,
    },

    #[error(
        "Component Group '{component_group}' flattened member count {actual} exceeds bound {maximum}"
    )]
    FlattenedMemberBoundExceeded {
        component_group: ComponentGroupSpecId,
        actual: usize,
        maximum: usize,
    },

    #[error("Component Group '{component_group}' has an invalid flattened member path: {source}")]
    InvalidMemberPath {
        component_group: ComponentGroupSpecId,
        #[source]
        source: ComponentGroupMemberPathError,
    },

    #[error(
        "Component Group '{component_group}' member '{member}' assigns Fleet-service purpose without a service-bearing leaf"
    )]
    InapplicableServicePurposeAssignment {
        component_group: ComponentGroupSpecId,
        member: ComponentGroupMemberId,
    },

    #[error(
        "Component Group '{component_group}' member '{member}' has {actual} labels; maximum is {maximum}"
    )]
    LabelBoundExceeded {
        component_group: ComponentGroupSpecId,
        member: ComponentGroupMemberId,
        actual: usize,
        maximum: usize,
    },

    #[error(
        "Component Group '{component_group}' member '{member}' label '{label}' is duplicated or not in canonical order"
    )]
    NonCanonicalLabelOrder {
        component_group: ComponentGroupSpecId,
        member: ComponentGroupMemberId,
        label: ComponentDeploymentLabelKey,
    },

    #[error(
        "Component Group '{component_group}' flattened member '{member_path:?}' repeats label key '{label}'"
    )]
    DuplicateEffectiveLabel {
        component_group: ComponentGroupSpecId,
        member_path: ComponentGroupMemberPath,
        label: ComponentDeploymentLabelKey,
    },

    #[error(
        "Component Group '{component_group}' flattened member '{member_path:?}' has {actual} effective labels; maximum is {maximum}"
    )]
    EffectiveLabelBoundExceeded {
        component_group: ComponentGroupSpecId,
        member_path: ComponentGroupMemberPath,
        actual: usize,
        maximum: usize,
    },

    #[error("Component Group graph canonical bytes {actual} exceed bound {maximum}")]
    CanonicalBytesBoundExceeded { actual: usize, maximum: usize },

    #[error("Component Group '{component_group}' is not in canonical order")]
    NonCanonicalGroupOrder {
        component_group: ComponentGroupSpecId,
    },

    #[error("Component Group '{component_group}' member '{member}' is not in canonical order")]
    NonCanonicalMemberOrder {
        component_group: ComponentGroupSpecId,
        member: ComponentGroupMemberId,
    },
}

fn checked_member_count(
    component_group: &ComponentGroupSpecId,
    source: &ComponentGroupSpecConfig,
) -> Result<usize, ComponentGroupTopologyError> {
    let count = source
        .components
        .len()
        .checked_add(source.groups.len())
        .ok_or_else(|| ComponentGroupTopologyError::MemberBoundExceeded {
            component_group: component_group.clone(),
            actual: usize::MAX,
            maximum: MAX_COMPONENT_GROUP_MEMBERS,
        })?;
    if count == 0 {
        return Err(ComponentGroupTopologyError::EmptyGroup {
            component_group: component_group.clone(),
        });
    }
    if count > MAX_COMPONENT_GROUP_MEMBERS {
        return Err(ComponentGroupTopologyError::MemberBoundExceeded {
            component_group: component_group.clone(),
            actual: count,
            maximum: MAX_COMPONENT_GROUP_MEMBERS,
        });
    }
    Ok(count)
}

fn compile_group(
    config: &ConfigModel,
    component_group: &ComponentGroupSpecId,
    source: &ComponentGroupSpecConfig,
) -> Result<ComponentGroupSpec, ComponentGroupTopologyError> {
    let mut seen = BTreeSet::new();
    let mut members = Vec::with_capacity(source.components.len() + source.groups.len());
    for (member, component) in &source.components {
        if !seen.insert(member.clone()) {
            return Err(ComponentGroupTopologyError::DuplicateMember {
                component_group: component_group.clone(),
                member: member.clone(),
            });
        }
        if !config
            .component_specs
            .contains_key(&component.component_spec)
        {
            return Err(ComponentGroupTopologyError::UnknownComponentSpec {
                component_group: component_group.clone(),
                member: member.clone(),
                component_spec: component.component_spec.clone(),
            });
        }
        members.push(ComponentGroupMember::Component {
            member: member.clone(),
            component_spec: component.component_spec.clone(),
            kind: component
                .service
                .clone()
                .map_or(ComponentGroupLeafKind::Ordinary, |service| {
                    ComponentGroupLeafKind::FleetService { service }
                }),
            service_purpose: component.service_purpose,
            labels: source_labels(&component.labels),
        });
    }
    for (member, included) in &source.groups {
        if !seen.insert(member.clone()) {
            return Err(ComponentGroupTopologyError::DuplicateMember {
                component_group: component_group.clone(),
                member: member.clone(),
            });
        }
        if !config
            .component_groups
            .contains_key(&included.component_group)
        {
            return Err(ComponentGroupTopologyError::UnknownIncludedGroup {
                component_group: component_group.clone(),
                included: included.component_group.clone(),
            });
        }
        members.push(ComponentGroupMember::Group {
            member: member.clone(),
            component_group: included.component_group.clone(),
            service_purpose: included.service_purpose,
            labels: source_labels(&included.labels),
        });
    }
    members.sort_by(|left, right| left.member().cmp(right.member()));
    Ok(ComponentGroupSpec {
        component_group: component_group.clone(),
        members,
    })
}

fn encode_u64(bytes: &mut Vec<u8>, value: usize) {
    let value = u64::try_from(value).expect("bounded Component Group length fits in u64");
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn encode_bytes(output: &mut Vec<u8>, value: &[u8]) {
    encode_u64(output, value.len());
    output.extend_from_slice(value);
}

fn encode_text(output: &mut Vec<u8>, value: &str) {
    encode_bytes(output, value.as_bytes());
}

fn encode_leaf_kind(output: &mut Vec<u8>, kind: &ComponentGroupLeafKind) {
    match kind {
        ComponentGroupLeafKind::Ordinary => output.push(0),
        ComponentGroupLeafKind::FleetService { service } => {
            output.push(1);
            encode_text(output, service.as_str());
        }
    }
}

fn encode_service_purpose(output: &mut Vec<u8>, purpose: Option<&FleetServiceMemberPurpose>) {
    match purpose {
        None => output.push(0),
        Some(FleetServiceMemberPurpose::Authority) => output.extend_from_slice(&[1, 0]),
        Some(FleetServiceMemberPurpose::Replica) => output.extend_from_slice(&[1, 1]),
        Some(FleetServiceMemberPurpose::PoolMember) => output.extend_from_slice(&[1, 2]),
    }
}

fn encode_labels(output: &mut Vec<u8>, labels: &[ComponentDeploymentLabel]) {
    encode_u64(output, labels.len());
    for label in labels {
        encode_text(output, label.key.as_str());
        encode_text(output, label.value.as_str());
    }
}

pub(super) fn source_labels(
    labels: &BTreeMap<ComponentDeploymentLabelKey, ComponentDeploymentLabelValue>,
) -> Vec<ComponentDeploymentLabel> {
    labels
        .iter()
        .map(|(key, value)| ComponentDeploymentLabel {
            key: key.clone(),
            value: value.clone(),
        })
        .collect()
}

fn validate_member_labels(
    component_group: &ComponentGroupSpecId,
    member: &ComponentGroupMember,
) -> Result<(), ComponentGroupTopologyError> {
    let labels = member.labels();
    if labels.len() > MAX_COMPONENT_DEPLOYMENT_LABELS {
        return Err(ComponentGroupTopologyError::LabelBoundExceeded {
            component_group: component_group.clone(),
            member: member.member().clone(),
            actual: labels.len(),
            maximum: MAX_COMPONENT_DEPLOYMENT_LABELS,
        });
    }
    let mut previous: Option<&ComponentDeploymentLabelKey> = None;
    for label in labels {
        if previous.is_some_and(|key| key >= &label.key) {
            return Err(ComponentGroupTopologyError::NonCanonicalLabelOrder {
                component_group: component_group.clone(),
                member: member.member().clone(),
                label: label.key.clone(),
            });
        }
        previous = Some(&label.key);
    }
    Ok(())
}

fn extend_effective_labels(
    component_group: &ComponentGroupSpecId,
    member_path: &ComponentGroupMemberPath,
    member_labels: &[ComponentDeploymentLabel],
    effective_labels: &mut BTreeMap<ComponentDeploymentLabelKey, ComponentDeploymentLabelValue>,
) -> Result<Vec<ComponentDeploymentLabelKey>, ComponentGroupTopologyError> {
    let mut added_label_keys = Vec::with_capacity(member_labels.len());
    for label in member_labels {
        match effective_labels.entry(label.key.clone()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(label.value.clone());
                added_label_keys.push(label.key.clone());
            }
            std::collections::btree_map::Entry::Occupied(_) => {
                return Err(ComponentGroupTopologyError::DuplicateEffectiveLabel {
                    component_group: component_group.clone(),
                    member_path: member_path.clone(),
                    label: label.key.clone(),
                });
            }
        }
    }
    if effective_labels.len() > MAX_COMPONENT_DEPLOYMENT_LABELS {
        return Err(ComponentGroupTopologyError::EffectiveLabelBoundExceeded {
            component_group: component_group.clone(),
            member_path: member_path.clone(),
            actual: effective_labels.len(),
            maximum: MAX_COMPONENT_DEPLOYMENT_LABELS,
        });
    }
    Ok(added_label_keys)
}
