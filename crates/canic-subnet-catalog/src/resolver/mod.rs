use crate::{
    CatalogError, SubnetCatalog, SubnetInfo, canonical_principal_text, parse_principal,
    principal_bytes,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, str::FromStr};

///
/// ResolveAs
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolveAs {
    Subnet,
    Canister,
}

impl ResolveAs {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Subnet => "subnet",
            Self::Canister => "canister",
        }
    }
}

impl FromStr for ResolveAs {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "subnet" => Ok(Self::Subnet),
            "canister" => Ok(Self::Canister),
            other => Err(format!("invalid value {other}; use subnet or canister")),
        }
    }
}

///
/// ResolvedSubnetSubject
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedSubnetSubject {
    Subnet,
    Canister,
}

impl ResolvedSubnetSubject {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Subnet => "subnet",
            Self::Canister => "canister",
        }
    }
}

///
/// ResolvedSubnet
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResolvedSubnet {
    pub input_principal: String,
    pub resolved_as: ResolvedSubnetSubject,
    pub resolved_from: String,
    pub subnet: SubnetInfo,
    pub matched_canister_principal: Option<String>,
    pub matched_routing_range: Option<crate::RoutingRange>,
}

impl SubnetCatalog {
    /// Resolve a principal as a known subnet or as a canister covered by a cached range.
    pub fn resolve_principal(
        &self,
        input: &str,
        forced: Option<ResolveAs>,
    ) -> Result<ResolvedSubnet, CatalogError> {
        let input_principal = canonical_principal_text(input)?;
        match forced {
            Some(ResolveAs::Subnet) => self.resolve_known_subnet(&input_principal),
            None if self.subnet_by_principal(&input_principal).is_some() => {
                self.resolve_known_subnet(&input_principal)
            }
            Some(ResolveAs::Canister) | None => self.resolve_canister(&input_principal),
        }
    }

    /// Resolve an exact principal or a unique cached subnet principal prefix.
    pub fn resolve_principal_or_prefix(
        &self,
        input: &str,
        forced: Option<ResolveAs>,
    ) -> Result<ResolvedSubnet, CatalogError> {
        if canonical_principal_text(input).is_ok() {
            return self.resolve_principal(input, forced);
        }
        self.resolve_principal_prefix(input, forced)
    }

    /// Resolve a unique prefix of a cached subnet principal.
    pub fn resolve_principal_prefix(
        &self,
        input: &str,
        forced: Option<ResolveAs>,
    ) -> Result<ResolvedSubnet, CatalogError> {
        let prefix = input.trim().to_ascii_lowercase();
        if prefix.is_empty() {
            return Err(CatalogError::PrincipalPrefixNotFound { prefix });
        }

        let matches = self.subnet_principal_prefix_matches(&prefix, forced);
        let mut iter = matches.iter();
        let Some(first) = iter.next() else {
            return Err(CatalogError::PrincipalPrefixNotFound { prefix });
        };
        if iter.next().is_some() {
            return Err(CatalogError::AmbiguousPrincipalPrefix {
                prefix,
                matches: matches
                    .iter()
                    .map(|subnet| format!("subnet:{subnet}"))
                    .collect::<Vec<_>>(),
            });
        }

        let mut resolved = self.resolve_known_subnet(first)?;
        resolved.input_principal = input.to_string();
        resolved.resolved_from = "subnet_principal_prefix".to_string();
        Ok(resolved)
    }

    fn subnet_principal_prefix_matches(
        &self,
        prefix: &str,
        forced: Option<ResolveAs>,
    ) -> BTreeSet<String> {
        let mut matches = BTreeSet::new();
        if forced != Some(ResolveAs::Canister) {
            for subnet in &self.subnets {
                if subnet.subnet_principal.starts_with(prefix) {
                    matches.insert(subnet.subnet_principal.clone());
                }
            }
        }
        matches
    }

    fn resolve_known_subnet(&self, input_principal: &str) -> Result<ResolvedSubnet, CatalogError> {
        let subnet = self
            .subnet_by_principal(input_principal)
            .cloned()
            .ok_or_else(|| CatalogError::UnknownSubnet {
                subnet_principal: input_principal.to_string(),
            })?;
        Ok(ResolvedSubnet {
            input_principal: input_principal.to_string(),
            resolved_as: ResolvedSubnetSubject::Subnet,
            resolved_from: "subnet_principal".to_string(),
            subnet,
            matched_canister_principal: None,
            matched_routing_range: None,
        })
    }

    /// Resolve a canister principal through cached routing ranges.
    pub fn resolve_canister(&self, input_principal: &str) -> Result<ResolvedSubnet, CatalogError> {
        let canonical_canister = parse_principal(input_principal, "canister_principal")?.to_text();
        let canister_bytes = principal_bytes(&canonical_canister, "canister_principal")?;
        let range = self
            .routing_ranges
            .iter()
            .find(|range| range_contains_principal(range, &canister_bytes).unwrap_or(false))
            .ok_or_else(|| CatalogError::RouteNotFound {
                canister_principal: canonical_canister.clone(),
                registry_version: self.registry_version,
                catalog_schema_version: self.catalog_schema_version,
            })?;
        let subnet = self
            .subnet_by_principal(&range.subnet_principal)
            .expect("catalog validation ensures routing subnet exists")
            .clone();
        Ok(ResolvedSubnet {
            input_principal: canonical_canister.clone(),
            resolved_as: ResolvedSubnetSubject::Canister,
            resolved_from: "routing_range".to_string(),
            subnet,
            matched_canister_principal: Some(canonical_canister),
            matched_routing_range: Some(range.clone()),
        })
    }
}

pub(crate) fn routing_range_sorts_after(start: &[u8], end: &[u8]) -> bool {
    start > end
}

fn range_contains_principal(
    range: &crate::RoutingRange,
    principal: &[u8],
) -> Result<bool, CatalogError> {
    let start = principal_bytes(&range.start_canister_id, "start_canister_id")?;
    let end = principal_bytes(&range.end_canister_id, "end_canister_id")?;
    Ok(start.as_slice() <= principal && principal <= end.as_slice())
}
