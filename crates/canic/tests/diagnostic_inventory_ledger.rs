//! Module: diagnostic_inventory_ledger
//!
//! Responsibility: guard the provisional 0.102 producer-coverage arithmetic.
//! Does not own: runtime diagnostics, numeric allocation or host catalogue data.
//! Boundary: reads checked-in B1 evidence and rejects coverage-ledger drift.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate directory should have a parent")
        .parent()
        .expect("workspace root should exist")
        .to_path_buf()
}

fn inventory_root() -> PathBuf {
    workspace_root().join("docs/audits/working/0.102-diagnostic-inventory")
}

fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn markdown_files(root: &Path) -> Vec<PathBuf> {
    let mut files = fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
        .map(|entry| {
            entry
                .expect("inventory directory entry should be readable")
                .path()
        })
        .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn uppercase_tokens(value: &str) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    let mut candidate = String::new();

    for character in value.chars().chain(std::iter::once(' ')) {
        if character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_' {
            candidate.push(character);
            continue;
        }

        if candidate.starts_with(|character: char| character.is_ascii_uppercase())
            && candidate.contains('_')
            && candidate.ends_with(|character: char| character.is_ascii_alphanumeric())
        {
            tokens.insert(std::mem::take(&mut candidate));
        } else {
            candidate.clear();
        }
    }

    tokens
}

fn first_table_cell(line: &str) -> Option<&str> {
    let mut cells = line.split('|');
    cells.next()?.is_empty().then(|| cells.next()).flatten()
}

fn table_tokens(source: &str, accepted_header: impl Fn(&str) -> bool) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    let mut accepted_table = false;

    for line in source.lines() {
        let Some(first_cell) = first_table_cell(line) else {
            accepted_table = false;
            continue;
        };
        let normalized_header = first_cell.trim().to_ascii_lowercase();
        if !first_cell.contains('`') && accepted_header(&normalized_header) {
            accepted_table = true;
            continue;
        }
        if !accepted_table || first_cell.trim_start().starts_with("---") {
            continue;
        }
        tokens.extend(uppercase_tokens(first_cell));
    }

    tokens
}

fn projection_tokens(root: &Path) -> BTreeSet<String> {
    table_tokens(&read(&root.join("projection-ledger.md")), |header| {
        header == "safe public projection"
    })
}

fn is_identity_column(header: &str) -> bool {
    let header = header.trim().to_ascii_lowercase();
    header.contains("candidate")
        || header.contains("exact identit")
        || header.contains("exact variant")
        || header == "exact internal producers"
        || header == "existing exact identity"
        || header == "public projection"
        || header == "projection"
        || header == "safe public projection"
}

fn is_owner_column(header: &str) -> bool {
    let header = header.trim().to_ascii_lowercase();
    header.contains("producer")
        || header.contains("source")
        || header.contains("typed owner")
        || header.contains("typed cause")
        || header.contains("typed decision")
        || header.contains("dependency")
        || header == "sites"
        || header == "effective sites"
        || header == "calls"
        || header == "status field"
}

fn materialized_table_tokens(source: &str) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    let mut identity_columns = Vec::new();

    for line in source.lines() {
        if !line.starts_with('|') {
            identity_columns.clear();
            continue;
        }
        let cells = line
            .split('|')
            .skip(1)
            .take_while(|cell| !cell.is_empty())
            .map(str::trim)
            .collect::<Vec<_>>();
        if cells.is_empty() {
            identity_columns.clear();
            continue;
        }
        if cells.iter().all(|cell| {
            cell.chars()
                .all(|character| matches!(character, '-' | ':' | ' '))
        }) {
            continue;
        }
        if cells.iter().any(|cell| is_identity_column(cell)) && !cells[0].contains('`') {
            identity_columns = cells
                .iter()
                .enumerate()
                .filter_map(|(index, cell)| is_identity_column(cell).then_some(index))
                .collect();
            continue;
        }
        for index in &identity_columns {
            if let Some(cell) = cells.get(*index) {
                tokens.extend(uppercase_tokens(cell));
            }
        }
    }

    tokens
}

fn materialized_inventory_tokens(root: &Path) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    for path in markdown_files(root) {
        tokens.extend(materialized_table_tokens(&read(&path)));
    }
    tokens
}

fn owned_table_tokens(source: &str) -> BTreeSet<String> {
    let mut owned = BTreeSet::new();
    let mut identity_columns = Vec::new();
    let mut owner_columns = Vec::new();

    for line in source.lines() {
        if !line.starts_with('|') {
            identity_columns.clear();
            owner_columns.clear();
            continue;
        }
        let cells = line
            .split('|')
            .skip(1)
            .take_while(|cell| !cell.is_empty())
            .map(str::trim)
            .collect::<Vec<_>>();
        if cells.is_empty() {
            identity_columns.clear();
            owner_columns.clear();
            continue;
        }
        if cells.iter().all(|cell| {
            cell.chars()
                .all(|character| matches!(character, '-' | ':' | ' '))
        }) {
            continue;
        }
        if cells.iter().any(|cell| is_identity_column(cell)) && !cells[0].contains('`') {
            identity_columns = cells
                .iter()
                .enumerate()
                .filter_map(|(index, cell)| is_identity_column(cell).then_some(index))
                .collect();
            owner_columns = cells
                .iter()
                .enumerate()
                .filter_map(|(index, cell)| is_owner_column(cell).then_some(index))
                .collect();
            continue;
        }
        let has_owner = owner_columns.iter().any(|index| {
            cells
                .get(*index)
                .is_some_and(|cell| !cell.is_empty() && !matches!(*cell, "none" | "self" | "—"))
        });
        if !has_owner {
            continue;
        }
        for index in &identity_columns {
            if let Some(cell) = cells.get(*index) {
                owned.extend(uppercase_tokens(cell));
            }
        }
    }

    owned
}

fn owned_inventory_tokens(root: &Path) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    for path in markdown_files(root) {
        tokens.extend(owned_table_tokens(&read(&path)));
    }
    tokens
}

fn cell_has_symbolic_source_anchor(cell: &str) -> bool {
    cell.split('`').skip(1).step_by(2).any(|candidate| {
        let candidate = candidate.trim();
        !candidate.is_empty()
            && !candidate.contains('/')
            && !candidate.contains(".rs")
            && candidate.chars().any(char::is_alphabetic)
            && (candidate.contains("::")
                || candidate.contains('_')
                || candidate.chars().any(char::is_uppercase))
    })
}

fn source_anchored_table_tokens(source: &str) -> BTreeSet<String> {
    let mut anchored = BTreeSet::new();
    let mut identity_columns = Vec::new();
    let mut owner_columns = Vec::new();

    for line in source.lines() {
        if !line.starts_with('|') {
            identity_columns.clear();
            owner_columns.clear();
            continue;
        }
        let cells = line
            .split('|')
            .skip(1)
            .take_while(|cell| !cell.is_empty())
            .map(str::trim)
            .collect::<Vec<_>>();
        if cells.is_empty() {
            identity_columns.clear();
            owner_columns.clear();
            continue;
        }
        if cells.iter().all(|cell| {
            cell.chars()
                .all(|character| matches!(character, '-' | ':' | ' '))
        }) {
            continue;
        }
        if cells.iter().any(|cell| is_identity_column(cell)) && !cells[0].contains('`') {
            identity_columns = cells
                .iter()
                .enumerate()
                .filter_map(|(index, cell)| is_identity_column(cell).then_some(index))
                .collect();
            owner_columns = cells
                .iter()
                .enumerate()
                .filter_map(|(index, cell)| is_owner_column(cell).then_some(index))
                .collect();
            continue;
        }
        let has_source_anchor = owner_columns.iter().any(|index| {
            cells
                .get(*index)
                .is_some_and(|cell| cell_has_symbolic_source_anchor(cell))
        });
        if !has_source_anchor {
            continue;
        }
        for index in &identity_columns {
            if let Some(cell) = cells.get(*index) {
                anchored.extend(uppercase_tokens(cell));
            }
        }
    }

    anchored
}

fn source_anchored_inventory_tokens(root: &Path) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    for path in markdown_files(root) {
        tokens.extend(source_anchored_table_tokens(&read(&path)));
    }
    tokens
}

fn producer_anchor_debt_by_file(root: &Path, missing: &BTreeSet<String>) -> Vec<(String, usize)> {
    let mut debt = markdown_files(root)
        .into_iter()
        .filter_map(|path| {
            let source = read(&path);
            let count = materialized_table_tokens(&source)
                .intersection(missing)
                .count();
            (count > 0).then(|| {
                (
                    path.file_name()
                        .expect("inventory evidence should have a file name")
                        .to_string_lossy()
                        .into_owned(),
                    count,
                )
            })
        })
        .collect::<Vec<_>>();
    debt.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    debt
}

fn identity_set_fingerprint(identities: &BTreeSet<String>) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    identities
        .iter()
        .flat_map(|identity| identity.bytes().chain(std::iter::once(0)))
        .fold(FNV_OFFSET_BASIS, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
        })
}

#[derive(Debug, Default)]
struct CompressionEvidence {
    actions: BTreeSet<String>,
    producers: BTreeSet<String>,
    projections: BTreeSet<String>,
    saw_public_self: bool,
}

const PUBLIC_INTENT_IDENTITIES: &[&str] = &[
    "INTENT_CONFLICT",
    "INTENT_EXPIRED",
    "INTENT_ID_EXHAUSTED",
    "INTENT_TRANSITION_INVALID",
    "INTENT_NOT_FOUND",
    "INTENT_SETTLEMENT_DUPLICATED",
    "INTENT_RESOURCE_TOTAL_CAPACITY_REACHED",
    "INTENT_EXPIRY_DEADLINE_OVERFLOW",
    "INTENT_RECEIPT_CONFLICT",
    "INTENT_RECEIPT_EVIDENCE_CONFLICT",
    "INTENT_RECEIPT_OWNERSHIP_MISMATCH",
    "INTENT_APPLICATION_RECEIPT_CAPACITY_UNAVAILABLE",
];

fn symbolic_anchors(cell: &str) -> BTreeSet<String> {
    cell.split('`')
        .skip(1)
        .step_by(2)
        .map(str::trim)
        .filter(|candidate| cell_has_symbolic_source_anchor(&format!("`{candidate}`")))
        .map(String::from)
        .collect()
}

fn source_compression_evidence(
    root: &Path,
    exact: &BTreeSet<String>,
    projection_identities: &BTreeSet<String>,
) -> BTreeMap<String, CompressionEvidence> {
    let mut evidence = exact
        .iter()
        .cloned()
        .map(|identity| (identity, CompressionEvidence::default()))
        .collect::<BTreeMap<_, _>>();

    for path in markdown_files(root) {
        let source = read(&path);
        let mut headers = Vec::new();
        for line in source.lines() {
            if !line.starts_with('|') {
                headers.clear();
                continue;
            }
            let cells = line
                .split('|')
                .skip(1)
                .take_while(|cell| !cell.is_empty())
                .map(str::trim)
                .collect::<Vec<_>>();
            if cells.is_empty()
                || cells.iter().all(|cell| {
                    cell.chars()
                        .all(|character| matches!(character, '-' | ':' | ' '))
                })
            {
                continue;
            }
            if cells.iter().any(|cell| is_identity_column(cell)) && !cells[0].contains('`') {
                headers = cells
                    .iter()
                    .map(|header| header.to_ascii_lowercase())
                    .collect();
                continue;
            }
            if headers.is_empty() {
                continue;
            }

            let identities = headers
                .iter()
                .enumerate()
                .filter(|(_, header)| is_identity_column(header) && !header.contains("projection"))
                .filter_map(|(index, _)| cells.get(index))
                .flat_map(|cell| uppercase_tokens(cell))
                .filter(|identity| exact.contains(identity))
                .collect::<BTreeSet<_>>();
            for identity in identities {
                let row = evidence
                    .get_mut(&identity)
                    .expect("exact identity should have a compression evidence row");
                for (index, header) in headers.iter().enumerate() {
                    let Some(cell) = cells.get(index) else {
                        continue;
                    };
                    if is_owner_column(header) {
                        row.producers.extend(symbolic_anchors(cell));
                    }
                    if header.contains("action") {
                        let action = cell.trim().replace('`', "");
                        if !action.is_empty() && action != "—" {
                            row.actions.insert(action);
                        }
                    }
                    if header.contains("projection") {
                        if cell.to_ascii_lowercase().contains("self") {
                            row.saw_public_self = true;
                        }
                        row.projections.extend(
                            uppercase_tokens(cell)
                                .into_iter()
                                .filter(|candidate| projection_identities.contains(candidate)),
                        );
                    }
                }
            }
        }
    }

    evidence
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ObservationExposure {
    Internal,
    Masked(String),
    Public,
    Unspecified,
}

#[derive(Debug)]
struct SourceObservation {
    action: Option<String>,
    class: Option<&'static str>,
    exposure: ObservationExposure,
    identity: String,
    producers: BTreeSet<String>,
    source: String,
}

fn diagnostic_class(cell: &str) -> Option<&'static str> {
    let normalized = cell
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_lowercase();
    [
        ("resourceexhausted", "resource_exhausted"),
        ("invalidinput", "invalid_input"),
        ("unauthorized", "unauthorized"),
        ("forbidden", "forbidden"),
        ("notfound", "not_found"),
        ("unavailable", "unavailable"),
        ("invariant", "invariant"),
        ("conflict", "conflict"),
        ("internal", "internal"),
    ]
    .into_iter()
    .find_map(|(needle, class)| normalized.starts_with(needle).then_some(class))
}

fn aggregate_projection(source: &str, identity: &str) -> Option<&'static str> {
    if source == "configuration-leaves.md"
        && identity.starts_with("CONFIG_")
        && !matches!(
            identity,
            "CONFIG_ALREADY_INITIALIZED" | "CONFIG_NOT_INITIALIZED"
        )
    {
        return Some("RUNTIME_CONFIGURATION_INVALID");
    }
    if source == "runtime-ops-leaves.md" && identity.starts_with("COMPONENT_DEPLOYMENT_") {
        return Some("COMPONENT_DEPLOYMENT_CONTEXT_INVALID");
    }
    if source == "fleet-activation-leaves.md" && is_fresh_fleet_activation_admission(identity) {
        return Some("FLEET_ACTIVATION_ADMISSION_INVALID");
    }
    if source == "intent-store-leaves.md"
        && identity.starts_with("INTENT_")
        && identity != "INTENT_STATE_INVALID"
        && !PUBLIC_INTENT_IDENTITIES.contains(&identity)
    {
        return Some("INTENT_STATE_INVALID");
    }
    None
}

#[expect(
    clippy::too_many_lines,
    reason = "one parser keeps the heterogeneous evidence-table normalization rules together"
)]
fn source_observations(root: &Path, exact: &BTreeSet<String>) -> Vec<SourceObservation> {
    let projection_targets = projection_observations(root)
        .into_keys()
        .collect::<BTreeSet<_>>();
    let mut observations = Vec::new();

    for path in markdown_files(root) {
        let source = read(&path);
        let file_name = path
            .file_name()
            .expect("inventory evidence should have a file name")
            .to_string_lossy();
        let mut headers = Vec::<String>::new();

        for (line_index, line) in source.lines().enumerate() {
            if !line.starts_with('|') {
                headers.clear();
                continue;
            }
            let cells = line
                .split('|')
                .skip(1)
                .take_while(|cell| !cell.is_empty())
                .map(str::trim)
                .collect::<Vec<_>>();
            if cells.is_empty()
                || cells.iter().all(|cell| {
                    cell.chars()
                        .all(|character| matches!(character, '-' | ':' | ' '))
                })
            {
                continue;
            }
            if cells.iter().any(|cell| is_identity_column(cell)) && !cells[0].contains('`') {
                headers = cells
                    .iter()
                    .map(|header| header.to_ascii_lowercase())
                    .collect();
                continue;
            }
            if headers.is_empty() {
                continue;
            }

            let identities = headers
                .iter()
                .enumerate()
                .filter(|(_, header)| is_identity_column(header) && !header.contains("projection"))
                .filter_map(|(index, _)| cells.get(index))
                .flat_map(|cell| uppercase_tokens(cell))
                .filter(|identity| exact.contains(identity))
                .collect::<BTreeSet<_>>();
            if identities.is_empty() {
                continue;
            }
            let producers = headers
                .iter()
                .enumerate()
                .filter(|(_, header)| is_owner_column(header))
                .filter_map(|(index, _)| cells.get(index))
                .flat_map(|cell| symbolic_anchors(cell))
                .collect::<BTreeSet<_>>();
            let projection_cells = headers
                .iter()
                .enumerate()
                .filter(|(_, header)| header.contains("projection"))
                .filter_map(|(index, _)| cells.get(index))
                .copied()
                .collect::<Vec<_>>();
            let targets = projection_cells
                .iter()
                .flat_map(|cell| uppercase_tokens(cell))
                .filter(|candidate| projection_targets.contains(candidate))
                .collect::<BTreeSet<_>>();
            let saw_self = projection_cells
                .iter()
                .any(|cell| cell.to_ascii_lowercase().contains("self"));
            let saw_public_observation = headers
                .iter()
                .enumerate()
                .filter(|(_, header)| {
                    header.contains("observation") || header.contains("observability")
                })
                .filter_map(|(index, _)| cells.get(index))
                .any(|cell| {
                    matches!(
                        cell.trim().to_ascii_lowercase().as_str(),
                        "public" | "public response" | "public status"
                    )
                });
            let saw_internal_observation = projection_cells.iter().any(|cell| {
                let cell = cell.to_ascii_lowercase();
                cell.contains("recent failure")
                    || cell.contains("internal only")
                    || cell.contains("operator only")
                    || cell.contains("lifecycle numeric")
                    || cell.contains("trap/log")
            });
            let exposure = match (targets.len(), saw_self || saw_public_observation) {
                (0, true) => ObservationExposure::Public,
                (1, false) => ObservationExposure::Masked(
                    targets
                        .iter()
                        .next()
                        .expect("one projection target should exist")
                        .clone(),
                ),
                _ => ObservationExposure::Unspecified,
            };
            let class = headers
                .iter()
                .enumerate()
                .filter(|(_, header)| header.contains("class"))
                .filter_map(|(index, _)| cells.get(index))
                .find_map(|cell| diagnostic_class(cell));
            let action = headers
                .iter()
                .enumerate()
                .find(|(_, header)| header.contains("action"))
                .and_then(|(index, _)| cells.get(index))
                .map(|cell| cell.trim().replace('`', ""))
                .filter(|cell| !cell.is_empty() && cell != "—");
            let source = format!("{file_name}:{}", line_index + 1);
            for identity in identities {
                let exposure = if exposure == ObservationExposure::Unspecified {
                    if projection_cells
                        .iter()
                        .flat_map(|cell| uppercase_tokens(cell))
                        .any(|projection| projection == identity)
                    {
                        ObservationExposure::Public
                    } else if saw_internal_observation {
                        ObservationExposure::Internal
                    } else {
                        aggregate_projection(&file_name, &identity)
                            .map_or(ObservationExposure::Unspecified, |projection| {
                                ObservationExposure::Masked(projection.to_string())
                            })
                    }
                } else {
                    exposure.clone()
                };
                observations.push(SourceObservation {
                    action: action.clone(),
                    class,
                    exposure,
                    identity,
                    producers: producers.clone(),
                    source: source.clone(),
                });
            }
        }
    }

    observations
}

fn apply_aggregate_projections(root: &Path, evidence: &mut BTreeMap<String, CompressionEvidence>) {
    let configuration = materialized_table_tokens(&read(&root.join("configuration-leaves.md")));
    for identity in configuration {
        if identity.starts_with("CONFIG_")
            && !matches!(
                identity.as_str(),
                "CONFIG_ALREADY_INITIALIZED" | "CONFIG_NOT_INITIALIZED"
            )
        {
            add_aggregate_projection(evidence, &identity, "RUNTIME_CONFIGURATION_INVALID");
        }
    }

    let runtime_ops = materialized_table_tokens(&read(&root.join("runtime-ops-leaves.md")));
    for identity in runtime_ops {
        if identity.starts_with("COMPONENT_DEPLOYMENT_")
            && identity != "COMPONENT_DEPLOYMENT_CONTEXT_INVALID"
        {
            add_aggregate_projection(evidence, &identity, "COMPONENT_DEPLOYMENT_CONTEXT_INVALID");
        }
    }

    let fleet_activation =
        materialized_table_tokens(&read(&root.join("fleet-activation-leaves.md")));
    for identity in fleet_activation {
        if is_fresh_fleet_activation_admission(&identity) {
            add_aggregate_projection(evidence, &identity, "FLEET_ACTIVATION_ADMISSION_INVALID");
        }
    }

    let intent_store = materialized_table_tokens(&read(&root.join("intent-store-leaves.md")));
    for identity in intent_store {
        if identity.starts_with("INTENT_")
            && identity != "INTENT_STATE_INVALID"
            && !PUBLIC_INTENT_IDENTITIES.contains(&identity.as_str())
        {
            add_aggregate_projection(evidence, &identity, "INTENT_STATE_INVALID");
        }
    }
}

fn compression_evidence(
    root: &Path,
    exact: &BTreeSet<String>,
    projection_identities: &BTreeSet<String>,
) -> BTreeMap<String, CompressionEvidence> {
    let mut evidence = source_compression_evidence(root, exact, projection_identities);
    apply_aggregate_projections(root, &mut evidence);
    evidence
}

fn add_aggregate_projection(
    evidence: &mut BTreeMap<String, CompressionEvidence>,
    identity: &str,
    projection: &str,
) {
    let Some(row) = evidence.get_mut(identity) else {
        return;
    };
    assert!(
        row.projections.is_empty() || row.projections.contains(projection),
        "aggregate projection conflicts with structured evidence: {identity}: {:?} versus {projection}",
        row.projections,
    );
    row.projections.insert(projection.to_string());
}

fn is_fresh_fleet_activation_admission(identity: &str) -> bool {
    identity.starts_with("FLEET_ACTIVATION_TOPOLOGY_")
        || matches!(
            identity,
            "FLEET_ACTIVATION_RELEASE_BUILD_MISMATCH"
                | "FLEET_ACTIVATION_AUTHORITY_EPOCH_INVALID"
                | "FLEET_ACTIVATION_APP_MISMATCH"
                | "FLEET_ACTIVATION_ROOT_PRINCIPAL_MISMATCH"
                | "FLEET_ACTIVATION_WASM_STORE_AUTHORITY_MISMATCH"
                | "FLEET_ACTIVATION_WASM_STORE_PRINCIPAL_INVALID"
                | "FLEET_ACTIVATION_WASM_STORE_PRINCIPAL_MISMATCH"
                | "FLEET_ACTIVATION_WASM_STORE_MODULE_HASH_ZERO"
        )
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum CompressionExposure {
    Internal,
    Masked(String),
    Public,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CompressionKey {
    origin: &'static str,
    subject: &'static str,
    condition: &'static str,
    class: &'static str,
    disposition: &'static str,
    remediation: &'static str,
    exposure: CompressionExposure,
}

#[derive(Debug, Default)]
struct CompressionGroup {
    coverage: BTreeSet<String>,
    identities: BTreeSet<String>,
    producers: BTreeSet<String>,
}

fn compression_subject(identity: &str) -> &'static str {
    fn category(subject: &str) -> Option<&'static str> {
        Some(match subject {
            "HASH" | "DIGEST" | "FINGERPRINT" | "CHECKSUM" => "digest",
            "TIME" | "TIMESTAMP" | "TTL" | "EXPIRY" | "DEADLINE" => "time",
            "COUNT" | "COUNTS" | "COUNTERS" | "CARDINALITY" | "LENGTH" | "SIZE" | "TOTAL"
            | "TOTALS" | "RATIO" => "count",
            "ID" | "IDENTITY" | "PRINCIPAL" | "CANISTER" | "ROLE" | "SPEC" | "PURPOSE" | "KIND"
            | "TYPE" | "LABEL" | "TAG" | "NAMESPACE" | "APP" | "FLEET" | "SUBNET" | "ROOT"
            | "COMPONENT" | "CHILD" | "PARENT" | "MEMBER" | "GROUP" | "SERVICE" | "CELL"
            | "SHARD" | "WORKER" | "WORKERS" | "TARGET" | "TARGETS" | "NONROOT" | "SELF" => {
                "identity"
            }
            "LIMIT" | "LIMITS" | "MAXIMUM" | "MINIMUM" | "BOUND" | "CAPACITY" | "BUDGET"
            | "QUOTA" | "THRESHOLD" | "HEADROOM" | "RANGE" => "capacity",
            "INDEX" | "CURSOR" | "ORDINAL" | "SEQUENCE" | "ORDER" | "OFFSET" | "PREFIX"
            | "STEP" | "TRAVERSAL" | "PATH" => "position",
            "VERSION" | "REVISION" | "GENERATION" | "EPOCH" => "version",
            "RECORD" | "STATE" | "STATUS" | "PROGRESS" | "PHASE" | "TRANSITION" | "ACTIVE"
            | "INACTIVE" | "PENDING" | "COMPLETE" | "COMPLETED" | "CURRENT" | "PREVIOUS"
            | "FINAL" | "INITIAL" | "END" | "START" => "state",
            "SET" | "LIST" | "LISTS" | "INVENTORY" | "MEMBERS" | "ROOTS" | "ROW" | "ROWS"
            | "ENTRIES" | "FIELDS" => "collection",
            "ENCODE" | "ENCODING" | "DECODE" | "DECODING" | "CODEC" | "JSON" | "HEX" | "FORMAT"
            | "PAYLOAD" | "SCHEMA" | "CANDIDATE" => "codec",
            "SOURCE" | "ORIGIN" | "PROVENANCE" => "origin",
            "REQUEST" | "INTENT" | "OPERATION" | "PLAN" | "COMMAND" | "QUERY" | "RETRY"
            | "REPLAY" | "RENEWAL" | "RECONCILIATION" => "request",
            "RESULT" | "RESPONSE" | "RECEIPT" | "RECEIPTS" | "EVIDENCE" | "OBSERVATION"
            | "CONFIRMATION" | "ACK" | "ACKNOWLEDGEMENTS" | "WITNESS" => "evidence",
            "BYTES" | "BYTE" | "FOOTPRINT" => "bytes",
            "ARTIFACT" | "MODULE" | "WASM" | "BUILD" | "RELEASE" | "MANIFEST" | "CHUNK"
            | "BUNDLE" => "artifact",
            "DIRECTORY" | "DIRECTORIES" | "REGISTRY" | "CATALOG" | "LEDGER" => "catalog",
            "CALLER" | "REQUESTER" | "ACTOR" | "SUBJECT" | "CONTROLLER" | "CONTROLLERS"
            | "ISSUER" | "COORDINATOR" | "OWNER" | "OWNERSHIP" | "AUDIENCE" | "AUDIENCES"
            | "RECIPIENT" | "RECEIVER" | "EXECUTOR" | "VERIFIER" | "AUTHORITY" | "BINDING" => {
                "authority"
            }
            "CONFIG" | "CONFIGURATION" | "POLICY" | "POLICIES" | "GRANT" | "GRANTS" | "SCOPE"
            | "SCOPES" | "CAPABILITY" | "PERMIT" | "WHITELIST" | "PREDICATE" | "FILTER"
            | "SELECTOR" | "ELIGIBILITY" | "ADMISSION" | "ADMISSIONS" | "RULE" | "OVERRIDE"
            | "MODE" => "configuration",
            "KEY" | "CERT" | "CERTIFICATE" | "PROOF" | "SIGNATURE" | "TOKEN" | "TOKENS"
            | "ATTESTATION" | "CREDENTIAL" | "CRYPTO" | "ALGORITHM" | "CHAIN" => "security",
            "CYCLES" | "BALANCE" | "FUNDING" | "COST" | "AMOUNT" | "RATE" | "CHARGE" | "BURN"
            | "ASSET" | "POOL" | "DEMAND" => "resource",
            "CALL" | "EFFECT" | "MANAGEMENT" | "ICP" | "DESTINATION" | "ROUTE" | "HOP"
            | "NETWORK" | "SYSTEM" => "platform",
            "MEMORY" | "SLOT" | "LAYOUT" | "WRITE" | "PERSIST" => "storage",
            "ACTIVATION" | "INSTALL" | "INSTALLATION" | "CREATION" | "CREATED" | "DELETION"
            | "DRAINING" | "REMOVAL" | "RECLAMATION" | "RECYCLING" | "ADOPTION" | "BOOTSTRAP"
            | "INIT" | "FINALIZATION" | "QUIESCENCE" | "SETTLEMENT" | "SYNCHRONIZATION"
            | "CONVERGENCE" | "LIFECYCLE" | "DEPLOYMENT" | "PUBLICATION" | "ADVANCE" | "COMMIT"
            | "COMMITMENT" | "RESERVATION" | "RESERVE" | "CLAIM" | "GC" => "lifecycle",
            "PLACEMENT" | "PLACEMENTS" | "TOPOLOGY" | "SHARDING" | "LINEAGE" | "ANCESTRY"
            | "BOUNDARY" => "topology",
            "ACCOUNTING" | "INTEGRITY" | "CANONICALIZATION" | "CONSTRUCTION" | "VALIDATION" => {
                "invariant"
            }
            _ => return None,
        })
    }

    let subject = identity
        .split('_')
        .rev()
        .skip(1)
        .find_map(category)
        .unwrap_or("general");
    match subject {
        "identity" | "origin" | "topology" => "authority",
        "bytes" | "count" | "resource" => "capacity",
        "invariant" => "state",
        _ => subject,
    }
}

fn compression_condition(identity: &str) -> &'static str {
    let segments = identity.split('_').collect::<Vec<_>>();
    let ending = segments.last().copied().unwrap_or("INVALID");
    let penultimate = segments
        .get(segments.len().saturating_sub(2))
        .copied()
        .unwrap_or_default();
    if matches!(penultimate, "BEFORE" | "AFTER" | "PRECEDES" | "OUTLIVES") {
        return "ordering";
    }
    if penultimate == "NOT" {
        return match ending {
            "FOUND" | "PRESENT" | "AVAILABLE" | "READY" | "RETAINED" => "unavailable",
            "ACTIVE" | "ENABLED" | "STOPPED" | "COMPLETE" => "inactive",
            _ => "invalid_state",
        };
    }
    match ending {
        "MISMATCH" | "CONFLICT" | "CHANGED" | "REGRESSED" | "STALE" => "conflict",
        "INVALID" | "NONCANONICAL" | "UNEXPECTED" | "UNKNOWN" | "ZERO" | "NONZERO" | "EMPTY" => {
            "invalid"
        }
        "MISSING" | "REQUIRED" | "UNAVAILABLE" | "UNREADY" | "UNPREPARED" | "FOUND" => {
            "unavailable"
        }
        "OVERFLOW" | "UNREPRESENTABLE" | "EXHAUSTED" | "EXCEEDED" | "MAXIMUM" | "LIMIT"
        | "RANGE" | "CAPACITY" | "HIGH" => "capacity",
        "UNDERFLOW" | "INSUFFICIENT" | "BELOW" => "insufficient",
        "DUPLICATE" | "DUPLICATED" | "REUSED" | "EXISTS" => "duplicate",
        "INCOMPLETE" | "PENDING" | "PROGRESS" | "UNCONVERGED" | "NONTERMINAL" | "REMAINS"
        | "STILL" => "incomplete",
        "INACTIVE" | "DISABLED" | "FENCED" | "BLOCKED" | "REJECTED" | "FORBIDDEN" => "inactive",
        "FAILED" | "LOST" => "failed",
        "EXPIRED" => "expired",
        "ANONYMOUS" => "unauthorized",
        "UNSUPPORTED" => "unsupported",
        "PRESENT" | "ACTIVE" | "COMPLETE" => "unexpected_state",
        _ => "invalid_state",
    }
}

fn compression_origin(identity: &str, subject: &str) -> &'static str {
    match subject {
        "configuration" => "configuration",
        "security" => "authentication",
        "platform" => "platform",
        "storage" => "storage",
        "artifact" => "artifact",
        "lifecycle" => "canister_lifecycle",
        "catalog" => "registry_directory",
        "authority" => "topology_authority",
        "codec" | "digest" => "canonical_state",
        "general" if identity.starts_with("ACCESS_") => "access",
        "general" if identity.starts_with("AUTH_") => "authentication",
        "general" if identity.starts_with("RPC_") => "rpc",
        "general" if identity.starts_with("RUNTIME_") => "runtime",
        "general" if identity.starts_with("BLOB_") => "blob",
        _ => "control_plane_state",
    }
}

fn compression_class(origin: &str, subject: &str, condition: &str, masked: bool) -> &'static str {
    if matches!(origin, "access" | "authentication")
        && matches!(subject, "authority" | "identity" | "security")
        && matches!(
            condition,
            "conflict" | "invalid" | "invalid_state" | "unauthorized"
        )
    {
        return "unauthorized";
    }
    if masked && matches!(condition, "conflict" | "invalid" | "invalid_state") {
        return "invariant";
    }
    match condition {
        "capacity" | "insufficient" => "resource_exhausted",
        "unauthorized" => "unauthorized",
        "unavailable" | "incomplete" | "inactive" | "expired" => "unavailable",
        "conflict" | "duplicate" | "unexpected_state" | "ordering" => "conflict",
        "failed" => "internal",
        _ => "invalid_input",
    }
}

fn compression_disposition(
    identity: &str,
    subject: &str,
    condition: &str,
    masked: bool,
) -> &'static str {
    if masked {
        return "reconcile";
    }
    if condition == "conflict"
        && identity
            .split('_')
            .any(|segment| matches!(segment, "OPERATION" | "REQUEST" | "INTENT" | "RETRY"))
    {
        return "exact_retry";
    }
    if subject == "platform" && condition == "failed" {
        return "bounded_retry";
    }
    match condition {
        "unavailable" | "capacity" | "insufficient" | "incomplete" | "inactive" | "expired" => {
            "retry_after_state_change"
        }
        "failed" => "reconcile",
        _ => "do_not_retry",
    }
}

fn compression_remediation(
    subject: &str,
    condition: &str,
    disposition: &str,
    masked: bool,
) -> &'static str {
    if masked {
        "state_reconciliation"
    } else if matches!(condition, "capacity" | "insufficient") {
        "capacity_relief"
    } else if subject == "security" {
        "credential_renewal"
    } else if subject == "platform" && condition == "failed" {
        "effect_recovery"
    } else if subject == "artifact" && !matches!(condition, "unavailable" | "conflict") {
        "reinstall"
    } else if disposition == "exact_retry" {
        "exact_replay"
    } else if matches!(
        condition,
        "unavailable" | "incomplete" | "inactive" | "expired"
    ) {
        "state_progression"
    } else {
        "request_correction"
    }
}

const AUTHORITY_REFRESH_ACTIONS: &[&str] = &[
    "reload",
    "re-fetch",
    "refetch",
    "refresh ",
    "refresh/",
    "re-read",
    "revalidate",
    "re-obtain",
    "resolve/query",
    "query exact",
    "query the exact",
    "query with the exact",
    "query as an admitted",
    "obtain live receiver",
    "renew against current registry",
];

const SECONDARY_AUTHORITY_REFRESH_ACTIONS: &[&str] = &[
    "query the asset",
    "query the current",
    "fetch the registry",
    "resolve current",
    "renew ",
];

const EXACT_REPLAY_ACTIONS: &[&str] = &[
    "exact retry",
    "exact-retry",
    "replay",
    "same operation",
    "same request",
    "original payload",
    "resume ",
    "resume/",
    "retry only the exact",
    "retry only retained",
    "retry unchanged operation",
    "roll over only",
];

const STATE_RECONCILIATION_ACTIONS: &[&str] = &[
    "reconcile",
    "inspect",
    "preserve",
    "recover",
    "restore",
    "repair",
    "re-resolve",
    "re-observe",
    "reobserve",
    "investigate",
];

const PHASE_ADVANCE_ACTIONS: &[&str] = &[
    "advance to",
    "advance the",
    "return the terminal phase",
    "finalize the provisioned result",
    "commit ",
    "adopt ",
    "activate ",
    "fence ",
    "accept or query",
    "begin/query",
    "cancel only",
    "claim, hand off",
    "claim/handoff",
    "collect ",
    "converge ",
    "finalize ",
    "hand off ",
    "initialize ",
    "persist ",
    "prepare ",
    "prepare/",
    "publish ",
    "reserve ",
    "reserve/",
    "return the exact initial",
    "record only the exact operation",
    "recycle through",
];

const STATE_PROGRESSION_ACTIONS: &[&str] = &[
    "complete",
    "finish",
    "wait",
    "await",
    "enable",
    "configure",
    "settle",
    "expire",
    "begin ",
    "continue",
    "change the named policy",
    "install ",
    "join ",
    "let bounded",
    "register ",
    "retry after",
    "retry in certified",
    "retry only after",
    "start a new",
    "after valid initialization",
];

const REQUEST_CORRECTION_ACTIONS: &[&str] = &[
    "correct",
    "supply",
    "provide",
    "choose",
    "invoke",
    "submit",
    "reject",
    "use ",
    "call ",
    "call from",
    "add or use",
    "allocate ",
    "assign ",
    "bind ",
    "deduplicate",
    "declare ",
    "generate ",
    "select ",
    "reduce ",
    "grant ",
    "import ",
    "include ",
    "issue ",
    "request ",
    "restrict ",
    "return qualified",
    "route through",
    "send ",
    "start root-only",
    "use/",
    "verify the canister",
];

const IMPLEMENTATION_CORRECTION_ACTIONS: &[&str] = &[
    "split ",
    "remove ",
    "replace ",
    "map ",
    "fix the maintained",
    "fix the owned",
    "fix the closed",
    "fix the eager",
    "fix canonical",
    "review ic/cdk",
    "review the pinned",
    "clear/restage",
    "deploy the role",
    "discard the prepared",
    "fix the admitted",
    "give every physical",
    "identify the exact runtime",
    "keep declarations",
    "move the declaration",
    "omit the impossible",
    "qualify the exact remote",
    "recompile ",
    "recompute ",
    "reconstruct ",
    "reduce/fix",
    "restage ",
    "restart ",
    "treat as implementation",
    "treat as internal schema",
];

const MANUAL_INTERVENTION_ACTIONS: &[&str] = &[
    "stop",
    "never",
    "do not retry",
    "donotretry",
    "fail closed",
    "cannot",
    "do not ",
    "no further",
    "treat as destructive",
    "treat the effect outcome",
];

fn contains_any(action: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| action.contains(needle))
}

fn action_remediation(action: &str) -> Option<&'static str> {
    let action = action.to_ascii_lowercase();

    if contains_any(&action, AUTHORITY_REFRESH_ACTIONS) {
        return Some("authority_refresh");
    }
    if contains_any(&action, EXACT_REPLAY_ACTIONS) {
        return Some("exact_replay");
    }
    if contains_any(&action, &["reinstall", "rebuild", "release set"]) {
        return Some("reinstall");
    }
    if contains_any(
        &action,
        &[
            "capacity", "quota", "maximum", "headroom", "budget", "free ", "free/", "top up",
            "top-up", "balance", "slot", "smaller", "reclaim", "prun",
        ],
    ) {
        return Some("capacity_relief");
    }
    let credential = contains_any(
        &action,
        &[
            "attestation",
            "certificate",
            "credential",
            "proof",
            "signature",
            "token",
            "witness",
        ],
    );
    let renewal = contains_any(
        &action,
        &[
            "acquire",
            "refresh",
            "renew",
            "reacquire",
            "reauthenticate",
            "reissue",
            "obtain",
            "provision",
            "prepare",
        ],
    );
    let configuration_action = contains_any(&action, &["configure", "configuration"]);
    if credential && renewal && !configuration_action {
        return Some("credential_renewal");
    }
    if contains_any(
        &action,
        &[
            "bounded retry",
            "bounded exact retry",
            "transport",
            "effect journal",
            "idempotent workflow",
            "retry only through the bounded",
        ],
    ) {
        return Some("effect_recovery");
    }
    if action.contains("inspect") && action.contains("receipt") {
        return Some("terminal_receipt_lookup");
    }
    if contains_any(&action, SECONDARY_AUTHORITY_REFRESH_ACTIONS) {
        return Some("authority_refresh");
    }
    if contains_any(&action, STATE_RECONCILIATION_ACTIONS) {
        return Some("state_reconciliation");
    }
    if contains_any(&action, PHASE_ADVANCE_ACTIONS) {
        return Some("phase_advance");
    }
    if contains_any(&action, STATE_PROGRESSION_ACTIONS) {
        return Some("state_progression");
    }
    if contains_any(&action, REQUEST_CORRECTION_ACTIONS) {
        return Some("request_correction");
    }
    if contains_any(&action, IMPLEMENTATION_CORRECTION_ACTIONS) {
        return Some("implementation_correction");
    }
    if contains_any(&action, MANUAL_INTERVENTION_ACTIONS) {
        return Some("manual_intervention");
    }
    None
}

fn disposition_for_remediation(remediation: &str) -> &'static str {
    match remediation {
        "exact_replay" => "exact_retry",
        "authority_refresh" | "capacity_relief" | "credential_renewal" | "phase_advance"
        | "reinstall" | "state_progression" => "retry_after_state_change",
        "effect_recovery" => "bounded_retry",
        "implementation_correction" | "manual_intervention" | "state_reconciliation" => "reconcile",
        "request_correction" | "terminal_receipt_lookup" => "do_not_retry",
        other => panic!("unexpected action remediation: {other}"),
    }
}

fn class_for_remediation(
    remediation: &str,
    derived: &'static str,
    explicit: Option<&'static str>,
) -> &'static str {
    explicit.unwrap_or(match remediation {
        "capacity_relief" => "resource_exhausted",
        "exact_replay" | "authority_refresh" | "phase_advance" => "conflict",
        "state_progression" => "unavailable",
        _ => derived,
    })
}

fn qualified_exposure(
    identity: &str,
    producer: &str,
    pair_exposures: &BTreeMap<(String, String), BTreeSet<ObservationExposure>>,
    identity_exposures: &BTreeMap<String, BTreeSet<ObservationExposure>>,
    evidence: &CompressionEvidence,
) -> CompressionExposure {
    let pair = (identity.to_string(), producer.to_string());
    let explicit = pair_exposures
        .get(&pair)
        .filter(|exposures| exposures.len() == 1)
        .or_else(|| {
            identity_exposures
                .get(identity)
                .filter(|exposures| exposures.len() == 1)
        })
        .and_then(|exposures| exposures.iter().next());
    match explicit {
        Some(ObservationExposure::Internal) => CompressionExposure::Internal,
        Some(ObservationExposure::Masked(projection)) => {
            CompressionExposure::Masked(projection.clone())
        }
        Some(ObservationExposure::Public) => CompressionExposure::Public,
        Some(ObservationExposure::Unspecified) => {
            unreachable!("unspecified exposure is excluded from explicit sets")
        }
        None => {
            assert!(
                evidence.projections.len() <= 1,
                "exact identity has conflicting public projections: {identity}: {:?}",
                evidence.projections
            );
            if let Some(projection) = evidence.projections.iter().next() {
                CompressionExposure::Masked(projection.clone())
            } else if evidence.saw_public_self {
                CompressionExposure::Public
            } else {
                CompressionExposure::Internal
            }
        }
    }
}

fn selected_singleton<T: Copy + Ord>(values: Option<&BTreeSet<T>>) -> Option<T> {
    values
        .filter(|values| values.len() == 1)
        .and_then(|values| values.iter().next().copied())
}

#[expect(
    clippy::too_many_lines,
    reason = "the complete grouping pass keeps source qualification and fail-closed fallback adjacent"
)]
fn compression_groups(
    root: &Path,
) -> (
    BTreeSet<String>,
    BTreeMap<String, CompressionEvidence>,
    BTreeMap<CompressionKey, CompressionGroup>,
) {
    let materialized = materialized_inventory_tokens(root);
    let projection_identities = projection_tokens(root);
    let exact = materialized
        .difference(&projection_identities)
        .cloned()
        .collect::<BTreeSet<_>>();
    let all_projection_identities = projection_identities
        .union(&exact)
        .cloned()
        .collect::<BTreeSet<_>>();
    let evidence = compression_evidence(root, &exact, &all_projection_identities);
    let observations = source_observations(root, &exact);
    let mut producer_pairs = BTreeSet::<(String, String)>::new();
    let mut pair_exposures = BTreeMap::<(String, String), BTreeSet<ObservationExposure>>::new();
    let mut identity_exposures = BTreeMap::<String, BTreeSet<ObservationExposure>>::new();
    let mut pair_remediations = BTreeMap::<(String, String), BTreeSet<&'static str>>::new();
    let mut identity_remediations = BTreeMap::<String, BTreeSet<&'static str>>::new();
    let mut pair_classes = BTreeMap::<(String, String), BTreeSet<&'static str>>::new();
    let mut identity_classes = BTreeMap::<String, BTreeSet<&'static str>>::new();
    for observation in observations {
        let remediation = observation.action.as_deref().and_then(action_remediation);
        assert!(
            observation.action.is_none() || remediation.is_some(),
            "source action lacks a canonical remediation: {}: {:?}",
            observation.source,
            observation.action,
        );
        if observation.exposure != ObservationExposure::Unspecified {
            identity_exposures
                .entry(observation.identity.clone())
                .or_default()
                .insert(observation.exposure.clone());
        }
        if let Some(remediation) = remediation {
            identity_remediations
                .entry(observation.identity.clone())
                .or_default()
                .insert(remediation);
        }
        if let Some(class) = observation.class {
            identity_classes
                .entry(observation.identity.clone())
                .or_default()
                .insert(class);
        }
        for producer in observation.producers {
            let pair = (observation.identity.clone(), producer);
            producer_pairs.insert(pair.clone());
            if observation.exposure != ObservationExposure::Unspecified {
                pair_exposures
                    .entry(pair.clone())
                    .or_default()
                    .insert(observation.exposure.clone());
            }
            if let Some(remediation) = remediation {
                pair_remediations
                    .entry(pair.clone())
                    .or_default()
                    .insert(remediation);
            }
            if let Some(class) = observation.class {
                pair_classes.entry(pair).or_default().insert(class);
            }
        }
    }
    assert!(
        pair_exposures
            .values()
            .all(|exposures| exposures.len() == 1),
        "one producer has conflicting explicit exposure contracts"
    );
    assert!(
        pair_remediations
            .values()
            .all(|remediations| remediations.len() == 1),
        "one producer has conflicting explicit action contracts"
    );
    assert!(
        pair_classes.values().all(|classes| classes.len() == 1),
        "one producer has conflicting explicit machine classes"
    );

    let mut groups = BTreeMap::<CompressionKey, CompressionGroup>::new();
    for (identity, producer) in producer_pairs {
        let row = &evidence[&identity];
        let subject = compression_subject(&identity);
        let condition = compression_condition(&identity);
        let origin = compression_origin(&identity, subject);
        let exposure = qualified_exposure(
            &identity,
            &producer,
            &pair_exposures,
            &identity_exposures,
            row,
        );
        let pair = (identity.clone(), producer.clone());
        let remediation = if matches!(
            &exposure,
            CompressionExposure::Masked(projection)
                if projection == "RUNTIME_CONFIGURATION_INVALID"
        ) {
            "reinstall"
        } else {
            selected_singleton(pair_remediations.get(&pair))
                .or_else(|| selected_singleton(identity_remediations.get(&identity)))
                .unwrap_or_else(|| {
                    let masked = !matches!(exposure, CompressionExposure::Public);
                    let derived_disposition =
                        compression_disposition(&identity, subject, condition, masked);
                    compression_remediation(subject, condition, derived_disposition, masked)
                })
        };
        let disposition = disposition_for_remediation(remediation);
        let derived_class = compression_class(
            origin,
            subject,
            condition,
            !matches!(exposure, CompressionExposure::Public),
        );
        let explicit_class = selected_singleton(pair_classes.get(&pair))
            .or_else(|| selected_singleton(identity_classes.get(&identity)));
        let key = CompressionKey {
            origin,
            subject,
            condition,
            class: class_for_remediation(remediation, derived_class, explicit_class),
            disposition,
            remediation,
            exposure,
        };
        let group = groups.entry(key).or_default();
        group.coverage.insert(format!("{identity} @ {producer}"));
        group.identities.insert(identity);
        group.producers.insert(producer);
    }
    (exact, evidence, groups)
}

#[derive(Debug, Default)]
struct ProjectionCatalogueRow {
    action: String,
    class: String,
    disposition: String,
    observation: String,
    summary: String,
}

#[derive(Debug)]
struct CompressionProposalRow {
    action: String,
    class: String,
    condition: String,
    coverage: BTreeSet<String>,
    disposition: String,
    exposure: &'static str,
    handling_key: String,
    label: String,
    observability: String,
    origin: String,
    producers: BTreeSet<String>,
    provisional_identities: BTreeSet<String>,
    projection: String,
    remediation: String,
    split_rationale: String,
    summary: String,
}

fn markdown_cells(line: &str) -> Option<Vec<&str>> {
    line.starts_with('|').then(|| {
        line.split('|')
            .skip(1)
            .take_while(|cell| !cell.is_empty())
            .map(str::trim)
            .collect()
    })
}

fn plain_markdown_cell(cell: &str) -> String {
    cell.trim().trim_matches('`').to_string()
}

fn projection_catalogue(root: &Path) -> BTreeMap<String, ProjectionCatalogueRow> {
    let source = read(&root.join("projection-ledger.md"));
    let projections = projection_tokens(root);
    let mut catalogue = projections
        .iter()
        .cloned()
        .map(|projection| (projection, ProjectionCatalogueRow::default()))
        .collect::<BTreeMap<_, _>>();
    let mut headers = Vec::<String>::new();

    for line in source.lines() {
        let Some(cells) = markdown_cells(line) else {
            headers.clear();
            continue;
        };
        if cells.is_empty()
            || cells.iter().all(|cell| {
                cell.chars()
                    .all(|character| matches!(character, '-' | ':' | ' '))
            })
        {
            continue;
        }
        if cells.first() == Some(&"Safe public projection") {
            headers = cells
                .iter()
                .map(|header| header.to_ascii_lowercase())
                .collect();
            continue;
        }
        if headers.is_empty() {
            continue;
        }

        let Some(projection) = cells
            .first()
            .and_then(|cell| uppercase_tokens(cell).into_iter().next())
        else {
            continue;
        };
        let Some(row) = catalogue.get_mut(&projection) else {
            continue;
        };
        for (index, header) in headers.iter().enumerate() {
            let Some(cell) = cells.get(index) else {
                continue;
            };
            let value = plain_markdown_cell(cell);
            if header.contains("host class") {
                row.class = value;
            } else if header.contains("numeric observation") {
                row.observation = value;
            } else if header.contains("host summary") {
                row.summary = value;
            } else if header == "disposition" {
                row.disposition = value;
            } else if header == "action" {
                row.action = value;
            }
        }
    }

    catalogue
}

fn projection_observations(root: &Path) -> BTreeMap<String, String> {
    let source = read(&root.join("projection-ledger.md"));
    let mut observations = BTreeMap::new();
    let mut observation_index = None;

    for line in source.lines() {
        let Some(cells) = markdown_cells(line) else {
            observation_index = None;
            continue;
        };
        if cells.is_empty()
            || cells.iter().all(|cell| {
                cell.chars()
                    .all(|character| matches!(character, '-' | ':' | ' '))
            })
        {
            continue;
        }
        if !cells[0].contains('`') {
            observation_index = cells
                .iter()
                .position(|header| header.to_ascii_lowercase().contains("numeric observation"));
            continue;
        }
        let Some(index) = observation_index else {
            continue;
        };
        let Some(identity) = uppercase_tokens(cells[0]).into_iter().next() else {
            continue;
        };
        let observation = cells
            .get(index)
            .map(|cell| plain_markdown_cell(cell))
            .expect("projection observation row should be complete");
        assert!(
            observations.insert(identity.clone(), observation).is_none(),
            "projection observation is duplicated: {identity}",
        );
    }

    observations
}

fn upper_snake(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn exact_group_label(key: &CompressionKey) -> String {
    let mut label = format!(
        "{}_{}_{}_{}_{}_{}",
        upper_snake(key.origin),
        upper_snake(key.subject),
        upper_snake(key.condition),
        upper_snake(key.class),
        upper_snake(key.disposition),
        upper_snake(key.remediation),
    );
    match &key.exposure {
        CompressionExposure::Internal => label.push_str("_INTERNAL_ONLY"),
        CompressionExposure::Masked(projection) => {
            label.push_str("_INTERNAL_FOR_");
            label.push_str(projection);
        }
        CompressionExposure::Public => {}
    }
    label
}

fn display_words(value: &str) -> String {
    value.replace('_', " ")
}

fn class_name(class: &str) -> String {
    match class {
        "invalid_input" => "InvalidInput",
        "unauthorized" => "Unauthorized",
        "forbidden" => "Forbidden",
        "not_found" => "NotFound",
        "conflict" => "Conflict",
        "resource_exhausted" => "ResourceExhausted",
        "unavailable" => "Unavailable",
        "invariant" => "Invariant",
        "internal" => "Internal",
        other => panic!("unknown diagnostic class: {other}"),
    }
    .to_string()
}

fn disposition_name(disposition: &str) -> String {
    match disposition {
        "do_not_retry" => "DoNotRetry",
        "exact_retry" => "ExactRetry",
        "retry_after_state_change" => "RetryAfterStateChange",
        "bounded_retry" => "BoundedRetry",
        "reconcile" => "Reconcile",
        other => panic!("unknown diagnostic disposition: {other}"),
    }
    .to_string()
}

fn action_for_remediation(remediation: &str) -> &'static str {
    match remediation {
        "state_reconciliation" => {
            "Inspect the correlated state evidence and reconcile before retry"
        }
        "capacity_relief" => "Increase or release the bounded capacity before retry",
        "credential_renewal" => "Acquire fresh credentials with the exact required authority",
        "effect_recovery" => {
            "Resume through the exact effect journal; do not repeat the effect blindly"
        }
        "reinstall" => "Rebuild and reinstall from one admitted release set",
        "exact_replay" => "Replay only the exact original request and operation identity",
        "authority_refresh" => "Refresh the exact current authority and retry from its head",
        "phase_advance" => "Advance through the next admitted workflow phase",
        "state_progression" => "Wait for or repair the required state transition before retry",
        "request_correction" => "Correct the request or authority evidence before retry",
        "implementation_correction" => {
            "Repair the maintained implementation contract before retrying"
        }
        "manual_intervention" => "Stop automatic retry and require operator intervention",
        "terminal_receipt_lookup" => {
            "Read the retained terminal receipt instead of repeating the completed operation"
        }
        other => panic!("unknown diagnostic remediation: {other}"),
    }
}

fn projection_remediation(disposition: &str) -> &'static str {
    match disposition {
        "DoNotRetry" => "request_correction",
        "ExactRetry" => "exact_replay",
        "RetryAfterStateChange" => "state_progression",
        "BoundedRetry" => "effect_recovery",
        "Reconcile" => "state_reconciliation",
        other => panic!("unknown projection disposition: {other}"),
    }
}

fn handling_key(
    origin: &str,
    subject: &str,
    condition: &str,
    class: &str,
    disposition: &str,
    remediation: &str,
    projection: &str,
) -> String {
    format!("{origin}/{subject}/{condition}/{class}/{disposition}/{remediation}/{projection}")
}

fn exact_proposal_rows(root: &Path) -> Vec<CompressionProposalRow> {
    let (_, _, groups) = compression_groups(root);
    let observations = projection_observations(root);

    groups
        .into_iter()
        .map(|(key, group)| {
            let class = class_name(key.class);
            let disposition = disposition_name(key.disposition);
            let projection = match &key.exposure {
                CompressionExposure::Internal => "none".to_string(),
                CompressionExposure::Masked(target) => target.clone(),
                CompressionExposure::Public => "self".to_string(),
            };
            let observability = match &key.exposure {
                CompressionExposure::Internal => {
                    "bounded internal status, recent failure or lifecycle log".to_string()
                }
                CompressionExposure::Masked(target) => {
                    observations
                        .get(target)
                        .unwrap_or_else(|| {
                            panic!("projection target lacks an observability owner: {target}")
                        })
                        .clone()
                }
                CompressionExposure::Public => "public".to_string(),
            };
            let summary = format!(
                "{} {} is {}",
                display_words(key.origin),
                display_words(key.subject),
                display_words(key.condition),
            );
            let split_rationale = if group.coverage.len() == 1 {
                format!(
                    "Only current observation requiring {disposition}/{} for {}/{}/{} with {projection} exposure",
                    key.remediation, key.origin, key.subject, key.condition,
                )
            } else {
                "shared handling contract".to_string()
            };
            CompressionProposalRow {
                action: action_for_remediation(key.remediation).to_string(),
                class: class.clone(),
                condition: format!("{} {}", key.subject, key.condition),
                coverage: group.coverage,
                disposition: disposition.clone(),
                exposure: match &key.exposure {
                    CompressionExposure::Internal => "internal; no public return boundary",
                    CompressionExposure::Masked(_) => "internal; projected before return",
                    CompressionExposure::Public => "safe public identity",
                },
                handling_key: handling_key(
                    key.origin,
                    key.subject,
                    key.condition,
                    &class,
                    &disposition,
                    key.remediation,
                    &projection,
                ),
                label: exact_group_label(&key),
                observability,
                origin: key.origin.to_string(),
                producers: group.producers,
                provisional_identities: group.identities,
                projection,
                remediation: key.remediation.to_string(),
                split_rationale,
                summary,
            }
        })
        .collect()
}

fn projection_proposal_rows(root: &Path) -> Vec<CompressionProposalRow> {
    let (_, _, groups) = compression_groups(root);
    let catalogue = projection_catalogue(root);

    catalogue
        .into_iter()
        .map(|(projection, catalogue_row)| {
            let inputs = groups
                .iter()
                .filter(|(key, _)| {
                    matches!(
                        &key.exposure,
                        CompressionExposure::Masked(target) if target == &projection
                    )
                })
                .flat_map(|(_, group)| group.identities.iter().cloned())
                .collect::<BTreeSet<_>>();
            let producers = groups
                .iter()
                .filter(|(key, _)| {
                    matches!(
                        &key.exposure,
                        CompressionExposure::Masked(target) if target == &projection
                    )
                })
                .flat_map(|(_, group)| group.producers.iter().cloned())
                .collect::<BTreeSet<_>>();
            let subject = compression_subject(&projection);
            let condition = compression_condition(&projection);
            let origin = compression_origin(&projection, subject);
            let remediation = projection_remediation(&catalogue_row.disposition);
            CompressionProposalRow {
                action: catalogue_row.action,
                class: catalogue_row.class.clone(),
                condition: format!("{subject} {condition}"),
                coverage: BTreeSet::from([format!("projection @ {projection}")]),
                disposition: catalogue_row.disposition.clone(),
                exposure: "safe public projection",
                handling_key: handling_key(
                    origin,
                    subject,
                    condition,
                    &catalogue_row.class,
                    &catalogue_row.disposition,
                    remediation,
                    "self",
                ),
                label: projection.clone(),
                observability: "public".to_string(),
                origin: origin.to_string(),
                producers,
                provisional_identities: BTreeSet::from([projection.clone()]),
                projection: "self".to_string(),
                remediation: remediation.to_string(),
                split_rationale: format!(
                    "Dedicated safe exposure boundary for {} masked exact observations",
                    inputs.len()
                ),
                summary: catalogue_row.summary,
            }
        })
        .collect()
}

fn origin_rank(origin: &str) -> usize {
    const ORDER: &[&str] = &[
        "access",
        "authentication",
        "rpc",
        "configuration",
        "runtime",
        "topology_authority",
        "registry_directory",
        "canister_lifecycle",
        "artifact",
        "funding",
        "platform",
        "storage",
        "canonical_state",
        "control_plane_state",
        "blob",
    ];
    ORDER
        .iter()
        .position(|candidate| *candidate == origin)
        .unwrap_or_else(|| panic!("unknown diagnostic origin: {origin}"))
}

fn compression_proposal_rows(root: &Path) -> Vec<CompressionProposalRow> {
    let mut rows = exact_proposal_rows(root);
    rows.extend(projection_proposal_rows(root));
    rows.sort_by(|left, right| {
        origin_rank(&left.origin)
            .cmp(&origin_rank(&right.origin))
            .then_with(|| left.label.cmp(&right.label))
            .then_with(|| left.handling_key.cmp(&right.handling_key))
    });
    rows
}

fn sanitized_tsv_cell(value: &str) -> String {
    value.replace(['\n', '\r', '\t'], " ").replace('|', "/")
}

fn render_compression_register(root: &Path) -> String {
    let rows = compression_proposal_rows(root);
    let mut rendered = String::from(
        "code|label|class|origin|disposition|summary|condition|handling_key|coverage|provisional_identities|producers|split_rationale|public_projection|observability|remediation|action|exposure\n",
    );
    for (index, row) in rows.iter().enumerate() {
        let cells = [
            (index + 1).to_string(),
            row.label.clone(),
            row.class.clone(),
            row.origin.clone(),
            row.disposition.clone(),
            row.summary.clone(),
            row.condition.clone(),
            row.handling_key.clone(),
            row.coverage.iter().cloned().collect::<Vec<_>>().join(";"),
            row.provisional_identities
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(","),
            row.producers.iter().cloned().collect::<Vec<_>>().join(","),
            row.split_rationale.clone(),
            row.projection.clone(),
            row.observability.clone(),
            row.remediation.clone(),
            row.action.clone(),
            row.exposure.to_string(),
        ];
        rendered.push_str(
            &cells
                .iter()
                .map(|cell| sanitized_tsv_cell(cell))
                .collect::<Vec<_>>()
                .join("|"),
        );
        rendered.push('\n');
    }
    rendered
}

fn compression_register_from_proposal() -> String {
    let source = read(
        &workspace_root().join("docs/design/0.102-compact-diagnostic-codes/allocation-proposal.md"),
    );
    let start_marker = "<!-- BEGIN GENERATED COMPRESSION REGISTER -->\n```text\n";
    let end_marker = "```\n<!-- END GENERATED COMPRESSION REGISTER -->";
    let (_, remainder) = source
        .split_once(start_marker)
        .expect("allocation proposal should contain the compression-register start marker");
    let (register, _) = remainder
        .split_once(end_marker)
        .expect("allocation proposal should contain the compression-register end marker");
    register.to_string()
}

fn update_compression_register_in_proposal(register: &str) {
    let path =
        workspace_root().join("docs/design/0.102-compact-diagnostic-codes/allocation-proposal.md");
    let source = read(&path);
    let start_marker = "<!-- BEGIN GENERATED COMPRESSION REGISTER -->\n```text\n";
    let end_marker = "```\n<!-- END GENERATED COMPRESSION REGISTER -->";
    let (prefix, remainder) = source
        .split_once(start_marker)
        .expect("allocation proposal should contain the compression-register start marker");
    let (_, suffix) = remainder
        .split_once(end_marker)
        .expect("allocation proposal should contain the compression-register end marker");
    let updated = format!("{prefix}{start_marker}{register}{end_marker}{suffix}");
    fs::write(&path, updated)
        .unwrap_or_else(|error| panic!("failed to update {}: {error}", path.display()));
}

fn consumer_rows(source: &str) -> BTreeSet<String> {
    let mut rows = BTreeSet::new();
    let mut consumer_table = false;

    for line in source.lines() {
        if !line.starts_with('|') {
            consumer_table = false;
            continue;
        }
        let cells = line
            .split('|')
            .skip(1)
            .take_while(|cell| !cell.is_empty())
            .map(str::trim)
            .collect::<Vec<_>>();
        if cells.is_empty() {
            consumer_table = false;
            continue;
        }
        if cells.first() == Some(&"Consumer function") {
            consumer_table = true;
            continue;
        }
        if !consumer_table
            || cells.iter().all(|cell| {
                cell.chars()
                    .all(|character| matches!(character, '-' | ':' | ' '))
            })
        {
            continue;
        }
        assert_eq!(cells.len(), 5, "consumer evidence row must have five cells");
        assert!(
            cells.iter().all(|cell| !cell.is_empty()),
            "consumer evidence row must be complete"
        );
        assert!(
            rows.insert(cells.join("\u{1f}")),
            "consumer evidence row must be unique"
        );
    }

    rows
}

#[test]
fn producer_coverage_counts_are_mechanically_reconciled() {
    let root = inventory_root();
    let projections = projection_tokens(&root);
    let materialized = materialized_inventory_tokens(&root);

    assert_eq!(projections.len(), 31, "safe-projection count drifted");
    assert!(
        projections.is_subset(&materialized),
        "every projection must be materialized in a coverage column",
    );
    assert_eq!(materialized.len(), 2_895, "structured-row count drifted");
    let exact_observation_count = materialized.difference(&projections).count();
    assert_eq!(
        exact_observation_count, 2_864,
        "exact producer-observation count drifted"
    );
    assert_eq!(
        identity_set_fingerprint(&materialized),
        12_019_081_722_552_986_691,
        "qualified coverage-label set drifted"
    );
}

#[test]
fn exact_producer_observations_name_structured_owner_evidence() {
    let root = inventory_root();
    let materialized = materialized_inventory_tokens(&root);
    let projections = projection_tokens(&root);
    let exact = materialized
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let owned = owned_inventory_tokens(&root);
    let missing = exact.difference(&owned).cloned().collect::<BTreeSet<_>>();

    assert_eq!(
        missing.len(),
        0,
        "structured owner-evidence debt changed; fingerprint: {}; examples: {:?}",
        identity_set_fingerprint(&missing),
        missing.iter().take(30).collect::<Vec<_>>(),
    );
}

#[test]
fn producer_symbol_anchor_debt_is_bounded() {
    let root = inventory_root();
    let materialized = materialized_inventory_tokens(&root);
    let projections = projection_tokens(&root);
    let exact = materialized
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_inventory_tokens(&root);
    let missing = exact
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        missing.len(),
        0,
        "producer symbol-anchor debt changed; fingerprint: {}; examples: {:?}; families: {:?}",
        identity_set_fingerprint(&missing),
        missing.iter().take(30).collect::<Vec<_>>(),
        producer_anchor_debt_by_file(&root, &missing)
            .into_iter()
            .take(20)
            .collect::<Vec<_>>(),
    );
    assert_eq!(
        identity_set_fingerprint(&missing),
        14_695_981_039_346_656_037,
        "producer symbol-anchor debt set drifted"
    );
}

#[test]
fn access_diagnostic_family_has_complete_symbolic_producer_anchors() {
    let source = read(&inventory_root().join("access-leaves.md"));
    let identities = materialized_table_tokens(&source);
    let anchored = source_anchored_table_tokens(&source);
    let projections = projection_tokens(&inventory_root());
    let exact = identities
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let missing = exact
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(exact.len(), 20, "access coverage-label count drifted");
    assert!(
        missing.is_empty(),
        "access producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn authority_restore_family_has_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let identities = materialized_inventory_tokens(&root)
        .into_iter()
        .filter(|identity| identity.starts_with("AUTHORITY_RESTORE_"))
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_inventory_tokens(&root);
    let missing = identities
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(identities.len(), 12, "authority-restore count drifted");
    assert!(
        missing.is_empty(),
        "authority-restore producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn production_diagnostic_consumers_are_structured_and_stable() {
    let rows = consumer_rows(&read(&inventory_root().join("public-boundary.md")));

    assert_eq!(rows.len(), 18, "production consumer count drifted");
    assert_eq!(
        identity_set_fingerprint(&rows),
        9_779_506_158_562_161_896,
        "production consumer manifest drifted"
    );
}

#[test]
fn authentication_direct_prose_family_has_complete_symbolic_producer_anchors() {
    const EXPECTED: [&str; 20] = [
        "AUTH_ACTIVE_DELEGATION_PROOF_MISSING",
        "AUTH_ATTESTATION_EXPIRY_OVERFLOW",
        "AUTH_ATTESTATION_RETRIEVAL_MISSING",
        "AUTH_CHAIN_KEY_CONFIG_FIXED_LENGTH_INVALID",
        "AUTH_CHAIN_KEY_CONFIG_HEX_INVALID",
        "AUTH_CHAIN_KEY_CONFIG_REQUIRED",
        "AUTH_CHAIN_KEY_DERIVATION_PATH_HASH_MISMATCH",
        "AUTH_CHAIN_KEY_POLICY_UNAVAILABLE",
        "AUTH_CHAIN_KEY_PUBLIC_KEY_INVALID",
        "AUTH_CHAIN_KEY_REVOCATION_LATENCY_ZERO",
        "AUTH_IC_ROOT_KEY_HEX_INVALID",
        "AUTH_IC_ROOT_KEY_LENGTH_INVALID",
        "AUTH_IC_ROOT_KEY_NETWORK_MISMATCH",
        "AUTH_IC_ROOT_KEY_REQUIRED",
        "AUTH_ROOT_CANISTER_PRINCIPAL_INVALID",
        "AUTH_ROOT_PROOF_RETRIEVAL_EXPIRED",
        "AUTH_ROOT_PROOF_RETRIEVAL_MISSING",
        "AUTH_TOKEN_RETENTION_ACTOR_CAPACITY",
        "AUTH_TOKEN_RETENTION_GLOBAL_CAPACITY",
        "AUTH_TOKEN_VERIFIER_DISABLED",
    ];

    let source = read(&inventory_root().join("auth-string-frontier.md"));
    let section = source
        .split_once("### Reconciled direct-prose additions")
        .expect("auth direct-prose section should exist")
        .1
        .split_once("## Same-Semantics Reuse And Wrapper Removal")
        .expect("auth direct-prose section should have a terminal heading")
        .0;
    let expected = EXPECTED
        .into_iter()
        .map(String::from)
        .collect::<BTreeSet<_>>();
    let identities = materialized_table_tokens(section);
    let anchored = source_anchored_table_tokens(section);

    assert_eq!(
        identities, expected,
        "auth direct-prose identity set drifted"
    );
    assert_eq!(
        anchored, expected,
        "auth direct-prose anchors are incomplete"
    );
}

#[test]
fn authentication_producer_symbol_anchor_debt_is_bounded() {
    let root = inventory_root();
    let projections = projection_tokens(&root);
    let identities = ["auth-policy-leaves.md", "auth-string-frontier.md"]
        .into_iter()
        .flat_map(|file| materialized_table_tokens(&read(&root.join(file))))
        .collect::<BTreeSet<_>>()
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_inventory_tokens(&root);
    let missing = identities
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        identities.len(),
        151,
        "authentication coverage-label count drifted"
    );
    assert_eq!(
        missing.len(),
        0,
        "authentication producer symbol-anchor debt changed; fingerprint: {}; examples: {:?}",
        identity_set_fingerprint(&missing),
        missing.iter().take(200).collect::<Vec<_>>(),
    );
}

#[test]
fn runtime_auth_renewal_and_admission_have_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("runtime-auth-renewal-admission-constructor-leaves.md"));
    let identities = materialized_table_tokens(&source)
        .difference(&projection_tokens(&root))
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = identities
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        identities.len(),
        13,
        "renewal/admission coverage-label count drifted"
    );
    assert!(
        missing.is_empty(),
        "renewal/admission producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn rpc_authorization_and_runtime_auth_have_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("rpc-authorization-runtime-auth-constructor-leaves.md"));
    let identities = materialized_table_tokens(&source)
        .difference(&projection_tokens(&root))
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = identities
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        identities.len(),
        11,
        "RPC/runtime-auth coverage-label count drifted"
    );
    assert!(
        missing.is_empty(),
        "RPC/runtime-auth producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn auth_prepare_replay_has_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("runtime-auth-prepare-replay-constructor-leaves.md"));
    let identities = materialized_table_tokens(&source)
        .difference(&projection_tokens(&root))
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = identities
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        identities.len(),
        21,
        "auth-prepare replay coverage-label count drifted"
    );
    assert!(
        missing.is_empty(),
        "auth-prepare replay producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn auth_prepare_and_provisioning_have_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("runtime-auth-prepare-provisioning-constructor-leaves.md"));
    let identities = materialized_table_tokens(&source)
        .difference(&projection_tokens(&root))
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = identities
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        identities.len(),
        8,
        "auth prepare/provisioning coverage-label count drifted"
    );
    assert!(
        missing.is_empty(),
        "auth prepare/provisioning producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn core_chain_key_batch_has_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("core-auth-constructor-leaves.md"));
    let identities = materialized_table_tokens(&source)
        .difference(&projection_tokens(&root))
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = identities
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        identities.len(),
        11,
        "core chain-key coverage-label count drifted"
    );
    assert!(
        missing.is_empty(),
        "core chain-key producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn canister_creation_funding_and_pool_helpers_have_symbolic_producer_anchors() {
    const EXPECTED: [&str; 5] = [
        "CANISTER_CREATION_FUNDING_OVERFLOW",
        "CANISTER_POOL_ASSET_NOT_REGISTERED",
        "CANISTER_POOL_CONFIG_CANISTER_CYCLES_ZERO",
        "CANISTER_POOL_CONFIG_MAXIMUM_BELOW_MINIMUM",
        "CANISTER_POOL_CONFIG_MINIMUM_ZERO",
    ];

    let root = inventory_root();
    let sources = [
        read(&root.join("canister-pool-constructor-leaves.md")),
        read(&root.join("final-small-adapter-constructor-leaves.md")),
    ];
    let expected = EXPECTED
        .into_iter()
        .map(String::from)
        .collect::<BTreeSet<_>>();
    let identities = sources
        .iter()
        .flat_map(|source| materialized_table_tokens(source))
        .collect::<BTreeSet<_>>();
    let anchored = sources
        .iter()
        .flat_map(|source| source_anchored_table_tokens(source))
        .collect::<BTreeSet<_>>();

    assert!(
        expected.is_subset(&identities),
        "creation-funding/pool-helper identity set is incomplete"
    );
    assert!(
        expected.is_subset(&anchored),
        "creation-funding/pool-helper producer anchors are incomplete"
    );
}

#[test]
fn canister_pool_initialization_reset_and_claim_have_complete_symbolic_producer_anchors() {
    const EXPECTED: [&str; 10] = [
        "CANISTER_POOL_CLAIM_ALLOCATION_MISMATCH",
        "CANISTER_POOL_CLAIM_DUPLICATE",
        "CANISTER_POOL_IMPORT_ASSET_CONFLICT",
        "CANISTER_POOL_IMPORT_MAXIMUM_EXCEEDED",
        "CANISTER_POOL_IMPORT_PRINCIPAL_DUPLICATE",
        "CANISTER_POOL_RECYCLE_WORKLOAD_REQUIRED",
        "CANISTER_POOL_RESET_COMPLETION_STATUS_INVALID",
        "CANISTER_POOL_RESET_FAILURE_STATUS_INVALID",
        "CANISTER_POOL_RESET_RETRY_STATUS_INVALID",
        "CANISTER_POOL_STORE_INVENTORY_CONFLICT",
    ];

    let source = read(&inventory_root().join("canister-pool-constructor-leaves.md"));
    let section = source
        .split_once("## Inventory Initialization, Reset And Claim Range")
        .expect("Canister pool initialization/reset/claim section should exist")
        .1
        .split_once("## Dynamic Public Context")
        .expect("Canister pool initialization/reset/claim section should terminate")
        .0;
    let expected = EXPECTED
        .into_iter()
        .map(String::from)
        .collect::<BTreeSet<_>>();
    let projections = projection_tokens(&inventory_root());
    let identities = materialized_table_tokens(section)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(section)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(identities, expected, "pool boundary identity set drifted");
    assert_eq!(anchored, expected, "pool boundary anchors are incomplete");
}

#[test]
fn canister_pool_autonomous_creation_has_complete_symbolic_producer_anchors() {
    const EXPECTED: [&str; 21] = [
        "CANISTER_POOL_BLOCKED_CREATION_NOT_PENDING",
        "CANISTER_POOL_CREATION_ALREADY_PENDING_CONFLICT",
        "CANISTER_POOL_CREATION_ATTEMPT_COST_AUTHORITY_PENDING",
        "CANISTER_POOL_CREATION_ATTEMPT_OPERATION_MISMATCH",
        "CANISTER_POOL_CREATION_ATTEMPT_PHASE_INVALID",
        "CANISTER_POOL_CREATION_BLOCK_COST_AUTHORITY_PENDING",
        "CANISTER_POOL_CREATION_CANCEL_KNOWN_UNAPPLIED_REQUIRED",
        "CANISTER_POOL_CREATION_COST_SETTLEMENT_MISMATCH",
        "CANISTER_POOL_CREATION_INVENTORY_CONFLICT",
        "CANISTER_POOL_CREATION_NOT_PENDING",
        "CANISTER_POOL_CREATION_PRINCIPAL_MISSING",
        "CANISTER_POOL_CREATION_RECEIPT_OPERATION_MISMATCH",
        "CANISTER_POOL_CREATION_RECEIPT_PRINCIPAL_MISMATCH",
        "CANISTER_POOL_CREATION_RETRY_BLOCKED_REQUIRED",
        "CANISTER_POOL_CREATION_RETRY_UNRESOLVED_EXPIRED_FORBIDDEN",
        "CANISTER_POOL_CREATION_ROLLOVER_KNOWN_UNAPPLIED_REQUIRED",
        "CANISTER_POOL_CREATION_SEQUENCE_EXHAUSTED",
        "CANISTER_POOL_CREATION_TERMINAL_EVIDENCE_CONFLICT",
        "CANISTER_POOL_CREATION_TERMINAL_PROGRESS_OVERWRITE",
        "CANISTER_POOL_CREATION_TIMESTAMP_EXHAUSTED",
        "CANISTER_POOL_CREATION_TIMESTAMP_NONMONOTONIC",
    ];

    let source = read(&inventory_root().join("canister-pool-constructor-leaves.md"));
    let section = source
        .split_once("## Autonomous Creation Intent Through Rollover")
        .expect("Canister pool autonomous-creation section should exist")
        .1
        .split_once("## Exclusive Asset Handoff")
        .expect("Canister pool autonomous-creation section should terminate")
        .0;
    let expected = EXPECTED
        .into_iter()
        .map(String::from)
        .collect::<BTreeSet<_>>();
    let projections = projection_tokens(&inventory_root());
    let identities = materialized_table_tokens(section)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(section)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(identities, expected, "creation identity set drifted");
    assert_eq!(anchored, expected, "creation anchors are incomplete");
}

#[test]
fn canister_pool_exclusive_handoff_has_complete_symbolic_producer_anchors() {
    const EXPECTED: [&str; 10] = [
        "CANISTER_POOL_HANDOFF_ALREADY_COMPLETE",
        "CANISTER_POOL_HANDOFF_ALREADY_PENDING",
        "CANISTER_POOL_HANDOFF_ASSET_AUTHORITY_MISMATCH",
        "CANISTER_POOL_HANDOFF_ASSET_STATUS_INVALID",
        "CANISTER_POOL_HANDOFF_COMPLETION_CANISTER_MISMATCH",
        "CANISTER_POOL_HANDOFF_COMPLETION_RECIPIENT_MISMATCH",
        "CANISTER_POOL_HANDOFF_CREATION_PENDING",
        "CANISTER_POOL_HANDOFF_JOURNAL_ASSET_MISMATCH",
        "CANISTER_POOL_HANDOFF_NOT_PENDING",
        "CANISTER_POOL_HANDOFF_RECEIPT_EXISTS",
    ];

    let source = read(&inventory_root().join("canister-pool-constructor-leaves.md"));
    let section = source
        .split_once("## Exclusive Asset Handoff")
        .expect("Canister pool handoff section should exist")
        .1
        .split_once("## Store Deletion, Configuration And Shared Helpers")
        .expect("Canister pool handoff section should terminate")
        .0;
    let expected = EXPECTED
        .into_iter()
        .map(String::from)
        .collect::<BTreeSet<_>>();
    let projections = projection_tokens(&inventory_root());
    let identities = materialized_table_tokens(section)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(section)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(identities, expected, "handoff identity set drifted");
    assert_eq!(anchored, expected, "handoff anchors are incomplete");
}

#[test]
fn canister_pool_store_configuration_and_helpers_have_complete_symbolic_producer_anchors() {
    const EXPECTED: [&str; 17] = [
        "CANISTER_POOL_ASSET_NOT_REGISTERED",
        "CANISTER_POOL_CONFIG_CANISTER_CYCLES_ZERO",
        "CANISTER_POOL_CONFIG_MAXIMUM_BELOW_MINIMUM",
        "CANISTER_POOL_CONFIG_MINIMUM_ZERO",
        "CANISTER_POOL_CREATION_ATTEMPT_COST_AUTHORITY_MISMATCH",
        "CANISTER_POOL_CREATION_ATTEMPT_PHASE_INVALID",
        "CANISTER_POOL_CREATION_COST_AUTHORITY_PENDING",
        "CANISTER_POOL_CREATION_INVENTORY_ADOPTION_MISSING",
        "CANISTER_POOL_CREATION_OPERATION_MISMATCH",
        "CANISTER_POOL_MAXIMUM_EXHAUSTED",
        "CANISTER_POOL_RECYCLE_COMPONENT_OWNER_MISMATCH",
        "CANISTER_POOL_RECYCLE_INVENTORY_STATUS_MISMATCH",
        "CANISTER_POOL_RECYCLE_RESET_NOT_TERMINAL",
        "CANISTER_POOL_STORE_DELETION_ORIGIN_MISMATCH",
        "CANISTER_POOL_STORE_DELETION_PENDING_AUTHORITY_MISMATCH",
        "CANISTER_POOL_STORE_DELETION_STATE_CONFLICT",
        "CANISTER_POOL_STORE_INVENTORY_CONFLICT",
    ];

    let source = read(&inventory_root().join("canister-pool-constructor-leaves.md"));
    let section = source
        .split_once("## Store Deletion, Configuration And Shared Helpers")
        .expect("Canister pool Store/configuration/helper section should exist")
        .1
        .split_once("## Mechanical Coverage")
        .expect("Canister pool Store/configuration/helper section should terminate")
        .0;
    let expected = EXPECTED
        .into_iter()
        .map(String::from)
        .collect::<BTreeSet<_>>();
    let projections = projection_tokens(&inventory_root());
    let identities = materialized_table_tokens(section)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(section)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(identities, expected, "pool helper identity set drifted");
    assert_eq!(anchored, expected, "pool helper anchors are incomplete");
}

#[test]
fn canister_pool_ops_has_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("canister-pool-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let identities = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = identities
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        identities.len(),
        56,
        "pool producer-coverage label count drifted"
    );
    assert!(
        missing.is_empty(),
        "pool anchors are incomplete: {missing:?}"
    );
}

#[test]
fn canister_pool_workflow_has_complete_symbolic_producer_anchors() {
    const EXPECTED: [&str; 18] = [
        "CANISTER_POOL_CREATION_COST_AUTHORITY_PENDING",
        "CANISTER_POOL_CREATION_CYCLES_LEDGER_MISMATCH",
        "CANISTER_POOL_CREATION_NOT_PENDING",
        "CANISTER_POOL_CREATION_PLACEMENT_SUBNET_MISMATCH",
        "CANISTER_POOL_CREATION_RECOVERY_DISAPPEARED",
        "CANISTER_POOL_CREATION_ROOT_MISMATCH",
        "CANISTER_POOL_HANDOFF_RECIPIENT_CONFLICT",
        "CANISTER_POOL_HANDOFF_RECIPIENT_INVALID",
        "CANISTER_POOL_HANDOFF_ROOT_NOT_DRAINING",
        "CANISTER_POOL_IMPORT_COMPONENT_MEMBER_FORBIDDEN",
        "CANISTER_POOL_IMPORT_DRAINING",
        "CANISTER_POOL_IMPORT_INFRASTRUCTURE_FORBIDDEN",
        "CANISTER_POOL_IMPORT_SUBNET_MISMATCH",
        "CANISTER_POOL_IMPORT_SUBNET_ROUTE_MISSING",
        "CANISTER_POOL_MAINTENANCE_ROOT_PHASE_INVALID",
        "CANISTER_POOL_MAXIMUM_EXHAUSTED",
        "CANISTER_POOL_REFILL_RETRY_DRAINING",
        "CANISTER_POOL_STATUS_LIMIT_INVALID",
    ];

    let root = inventory_root();
    let source = read(&root.join("canister-pool-workflow-constructor-leaves.md"));
    let expected = EXPECTED
        .into_iter()
        .map(String::from)
        .collect::<BTreeSet<_>>();
    let projections = projection_tokens(&root);
    let identities = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(identities, expected, "pool workflow identity set drifted");
    assert_eq!(anchored, expected, "pool workflow anchors are incomplete");
}

#[test]
fn root_store_bootstrap_has_complete_symbolic_producer_anchors() {
    const EXPECTED: [&str; 22] = [
        "ROOT_STORE_ADMISSIONS_TOPOLOGY_DIGEST_MISMATCH",
        "ROOT_STORE_ARTIFACT_PATH_MISSING",
        "ROOT_STORE_ARTIFACT_SHA256_FORMAT_INVALID",
        "ROOT_STORE_ARTIFACT_SIZE_ZERO",
        "ROOT_STORE_BYTE_CAPACITY_EXCEEDED",
        "ROOT_STORE_CATALOG_PAYLOAD_HASH_INVALID",
        "ROOT_STORE_CATALOG_RAW_MODULE_HASH_MISSING",
        "ROOT_STORE_LIVE_CATALOG_MISMATCH",
        "ROOT_STORE_RAW_MODULE_HASH_CONFLICT",
        "ROOT_STORE_RELEASE_SET_BUILD_MISMATCH",
        "ROOT_STORE_RELEASE_SET_BYTES_NONCANONICAL",
        "ROOT_STORE_RELEASE_SET_BYTES_OVERFLOW",
        "ROOT_STORE_RELEASE_SET_CANONICALIZATION_FAILED",
        "ROOT_STORE_RELEASE_SET_DIGEST_MISMATCH",
        "ROOT_STORE_RELEASE_SET_ENTRY_AUTHORITY_MISMATCH",
        "ROOT_STORE_RELEASE_SET_ENTRY_COUNT_MISMATCH",
        "ROOT_STORE_RELEASE_SET_JSON_INVALID",
        "ROOT_STORE_RELEASE_SET_MANIFEST_SIZE_INVALID",
        "ROOT_STORE_RELEASE_SET_TOPOLOGY_MISMATCH",
        "ROOT_STORE_ROLE_ARTIFACT_CONFLICT",
        "ROOT_STORE_STAGED_ARTIFACT_AUTHORITY_MISMATCH",
        "ROOT_STORE_STAGED_ROLE_ARTIFACT_CONFLICT",
    ];

    let root = inventory_root();
    let source = read(&root.join("root-store-bootstrap-constructor-leaves.md"));
    let expected = EXPECTED
        .into_iter()
        .map(String::from)
        .collect::<BTreeSet<_>>();
    let projections = projection_tokens(&root);
    let identities = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(identities, expected, "root Store identity set drifted");
    assert_eq!(anchored, expected, "root Store anchors are incomplete");
}

#[test]
fn root_bootstrap_and_store_state_have_complete_symbolic_producer_anchors() {
    const EXPECTED: [&str; 9] = [
        "ACCESS_BUILD_NETWORK_UNAVAILABLE",
        "ROOT_SUBNET_DISCOVERY_EMPTY",
        "WASM_STORE_ADOPTION_AUTHORITY_CONFLICT",
        "WASM_STORE_ADOPTION_AUTHORITY_INVALID",
        "WASM_STORE_ADOPTION_INTENT_MISSING",
        "WASM_STORE_ADOPTION_INVENTORY_ALREADY_POPULATED",
        "WASM_STORE_ADOPTION_NOT_VERIFIED",
        "WASM_STORE_ADOPTION_RECEIPT_AUTHORITY_MISMATCH",
        "WASM_STORE_ADOPTION_VERIFIED_TIME_MISSING",
    ];

    let root = inventory_root();
    let source = read(&root.join("root-bootstrap-store-state-constructor-leaves.md"));
    let expected = EXPECTED
        .into_iter()
        .map(String::from)
        .collect::<BTreeSet<_>>();
    let projections = projection_tokens(&root);
    let identities = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        identities, expected,
        "root/Store state identity set drifted"
    );
    assert_eq!(
        anchored, expected,
        "root/Store state anchors are incomplete"
    );
}

#[test]
fn wasm_store_lifecycle_has_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("wasm-store-lifecycle-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(labels.len(), 21, "Store lifecycle coverage count drifted");
    assert!(
        missing.is_empty(),
        "Store lifecycle producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn fleet_registry_mirror_has_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("fleet-registry-mirror-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(labels.len(), 50, "Fleet Mirror coverage count drifted");
    assert!(
        missing.is_empty(),
        "Fleet Mirror producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn component_directory_and_fleet_peer_have_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("component-directory-peer-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        43,
        "Component Directory/Fleet peer coverage count drifted"
    );
    assert!(
        missing.is_empty(),
        "Component Directory/Fleet peer producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn component_directory_synchronization_ops_have_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("component-directory-synchronization-ops-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        55,
        "Component Directory synchronization ops coverage count drifted"
    );
    assert!(
        missing.is_empty(),
        "Component Directory synchronization ops anchors are incomplete: {missing:?}"
    );
}

#[test]
fn fleet_activation_and_scaling_have_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("fleet-activation-scaling-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        5,
        "Fleet activation/scaling coverage count drifted"
    );
    assert!(
        missing.is_empty(),
        "Fleet activation/scaling producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn core_small_ops_have_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("core-small-ops-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(labels.len(), 8, "core small-ops coverage count drifted");
    assert!(
        missing.is_empty(),
        "core small-ops producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn small_workflows_have_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("small-workflow-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(labels.len(), 6, "small-workflow coverage count drifted");
    assert!(
        missing.is_empty(),
        "small-workflow producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn final_small_adapters_have_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("final-small-adapter-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        28,
        "final small-adapter coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "final small-adapter producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn component_runtime_has_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("component-runtime-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        74,
        "Component runtime coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "Component runtime producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn runtime_auth_root_issuer_batch_has_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("runtime-auth-root-issuer-batch-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        1,
        "root issuer/batch coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "root issuer/batch producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn root_and_nonroot_lifecycle_have_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("runtime-root-nonroot-lifecycle-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        2,
        "root/non-root lifecycle coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "root/non-root lifecycle producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn runtime_coordination_restore_and_activation_have_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("runtime-coordination-restore-activation-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        12,
        "runtime coordination/restore/activation coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "runtime coordination/restore/activation producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn environment_component_rpc_and_service_binding_have_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source =
        read(&root.join("environment-component-rpc-service-binding-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        11,
        "environment/Component RPC/service-binding coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "environment/Component RPC/service-binding producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn cascade_refill_and_intent_storage_have_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("cascade-refill-intent-storage-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        11,
        "cascade/refill/intent-storage coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "cascade/refill/intent-storage producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn authority_restore_and_placement_allocation_have_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("authority-restore-placement-allocation-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        23,
        "authority-restore/placement-allocation coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "authority-restore/placement-allocation producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn icp_refill_replay_has_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("icp-refill-replay-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        15,
        "ICP-refill replay coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "ICP-refill replay producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn runtime_intent_and_rpc_execution_have_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("runtime-intent-rpc-execution-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        17,
        "runtime-intent/RPC-execution coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "runtime-intent/RPC-execution producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn publication_binding_and_release_have_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("publication-binding-release-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        11,
        "publication binding/release coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "publication binding/release producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn publication_gc_has_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("publication-gc-error-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        62,
        "publication GC coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "publication GC producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn publication_transport_has_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("publication-transport-error-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        22,
        "publication transport coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "publication transport producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn rpc_workflow_has_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("rpc-workflow-error-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        23,
        "RPC workflow coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "RPC workflow producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn memory_adapter_has_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("memory-adapter-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        74,
        "memory-adapter coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "memory-adapter producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn fleet_activation_has_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("fleet-activation-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        30,
        "Fleet-activation coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "Fleet-activation producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn fleet_coordinator_workflow_has_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("fleet-coordinator-workflow-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        24,
        "Fleet Coordinator workflow coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "Fleet Coordinator workflow producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn fleet_coordinator_deployment_ledger_has_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("fleet-coordinator-deployment-ledger-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        51,
        "Fleet Coordinator deployment-ledger coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "Fleet Coordinator deployment-ledger producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn fleet_subnet_root_workflow_has_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("fleet-subnet-root-workflow-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        58,
        "Fleet Subnet Root workflow coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "Fleet Subnet Root workflow producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn component_provisioning_workflow_has_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("component-provisioning-workflow-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        79,
        "Component provisioning workflow coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "Component provisioning workflow producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn component_registry_workflow_has_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("component-registry-workflow-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        239,
        "Component Registry workflow coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "Component Registry workflow producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn component_registry_ops_has_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("component-registry-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        449,
        "Component Registry ops coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "Component Registry ops producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn fleet_coordinator_root_deletion_has_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("fleet-coordinator-root-deletion-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        101,
        "Fleet Coordinator root-deletion coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "Fleet Coordinator root-deletion producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn fleet_coordinator_direct_constructors_have_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("fleet-coordinator-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        200,
        "Fleet Coordinator direct-constructor coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "Fleet Coordinator direct-constructor producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn fleet_coordinator_receipt_frontier_has_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("fleet-coordinator-receipt-invariant-frontier.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        327,
        "Fleet Coordinator receipt-frontier coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "Fleet Coordinator receipt-frontier producer anchors are incomplete: {missing:?}"
    );
}

#[test]
fn component_provisioning_direct_constructors_have_complete_symbolic_producer_anchors() {
    let root = inventory_root();
    let source = read(&root.join("component-provisioning-constructor-leaves.md"));
    let projections = projection_tokens(&root);
    let labels = materialized_table_tokens(&source)
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let anchored = source_anchored_table_tokens(&source);
    let missing = labels
        .difference(&anchored)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels.len(),
        230,
        "Component provisioning direct-constructor coverage count drifted: {labels:?}"
    );
    assert!(
        missing.is_empty(),
        "Component provisioning direct-constructor producer anchors are incomplete: {missing:?}"
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the allocation gate independently asserts every density and bijection invariant"
)]
fn diagnostic_compression_map_is_complete_and_dense() {
    let root = inventory_root();
    let materialized = materialized_inventory_tokens(&root);
    let projections = projection_tokens(&root);
    let (exact, evidence, groups) = compression_groups(&root);
    let exact_projection_targets = groups
        .keys()
        .filter_map(|key| match &key.exposure {
            CompressionExposure::Masked(projection) => Some(projection),
            CompressionExposure::Internal | CompressionExposure::Public => None,
        })
        .filter(|projection| exact.contains(*projection))
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        exact_projection_targets,
        BTreeSet::from([
            "COMPONENT_ALLOCATION_CAPACITY_EXHAUSTED".to_string(),
            "COMPONENT_CHILD_PARENT_ROLE_CAPACITY_EXHAUSTED".to_string(),
            "COMPONENT_DESCENDANT_CAPACITY_EXHAUSTED".to_string(),
            "COMPONENT_PROVISIONING_COUNT_OVERFLOW".to_string(),
            "COMPONENT_PROVISIONING_ROOT_GROUP_PLACEMENT_CAPACITY_EXCEEDED".to_string(),
            "COMPONENT_SPEC_ALLOCATION_CAPACITY_EXHAUSTED".to_string(),
            "FLEET_ACTIVATION_STATE_INVALID".to_string(),
            "PEER_COMPONENT_CAPACITY_EXHAUSTED".to_string(),
        ]),
        "exact identities reused as projection targets drifted",
    );
    assert!(
        groups
            .keys()
            .filter_map(|key| match &key.exposure {
                CompressionExposure::Masked(projection) => Some(projection),
                CompressionExposure::Internal | CompressionExposure::Public => None,
            })
            .all(|projection| materialized.contains(projection)),
        "every public projection must be an allocated coverage identity",
    );
    assert!(
        projections.iter().all(|projection| evidence
            .values()
            .any(|row| row.projections.contains(projection))),
        "every projection-only code must have at least one exact mapped input",
    );
    let rows = compression_proposal_rows(&root);

    assert_eq!(exact.len(), 2_864, "exact observation count drifted");
    assert_eq!(evidence.len(), 2_864, "exact evidence count drifted");
    assert_eq!(groups.len(), 960, "exact compression-group count drifted");
    assert_eq!(projections.len(), 31, "projection-only count drifted");
    assert_eq!(rows.len(), 991, "dense initial allocation count drifted");
    assert!(
        rows.len() < 1_000,
        "four-digit initial allocation fails the compression gate: {} exact groups + {} projections = {} rows",
        groups.len(),
        projections.len(),
        rows.len(),
    );
    assert!(u16::try_from(rows.len()).is_ok());

    let exact_coverage = source_observations(&root, &exact)
        .into_iter()
        .flat_map(|observation| {
            observation
                .producers
                .into_iter()
                .map(move |producer| format!("{} @ {producer}", observation.identity))
        })
        .collect::<BTreeSet<_>>();
    let expected_coverage = exact_coverage
        .iter()
        .cloned()
        .chain(
            projections
                .iter()
                .map(|projection| format!("projection @ {projection}")),
        )
        .collect::<BTreeSet<_>>();
    let mut coverage = BTreeSet::new();
    let mut labels = BTreeSet::new();
    for row in &rows {
        assert!(
            !row.producers.is_empty(),
            "code has no producer: {}",
            row.label
        );
        assert!(
            labels.insert(row.label.clone()),
            "canonical code label is duplicated: {}",
            row.label
        );
        for observation in &row.coverage {
            assert!(
                coverage.insert(observation.clone()),
                "qualified coverage observation maps to more than one code: {observation}"
            );
        }
    }
    assert_eq!(
        coverage, expected_coverage,
        "coverage-to-code map does not cover the exact qualified frontier"
    );
    assert_eq!(
        exact_coverage.len(),
        3_898,
        "producer coverage count drifted"
    );
    assert_eq!(
        expected_coverage.len(),
        3_929,
        "total coverage count drifted"
    );
    assert_eq!(
        groups
            .values()
            .filter(|group| group.coverage.len() == 1)
            .count(),
        472,
        "exact singleton count drifted",
    );
    assert_eq!(
        rows.iter().filter(|row| row.coverage.len() == 1).count(),
        503,
        "total singleton allocation count drifted",
    );
}

#[test]
fn high_risk_action_and_exposure_contracts_remain_distinct() {
    let root = inventory_root();
    let rows = compression_proposal_rows(&root);
    let rows_for = |identity: &str| {
        let prefix = format!("{identity} @ ");
        rows.iter()
            .filter(|row| {
                row.coverage
                    .iter()
                    .any(|coverage| coverage.starts_with(&prefix))
            })
            .collect::<Vec<_>>()
    };

    let stale_authority = rows_for("COMPONENT_CHILD_REGISTRY_AUTHORITY_STALE");
    assert_eq!(stale_authority.len(), 1);
    assert_eq!(stale_authority[0].remediation, "authority_refresh");
    assert_eq!(stale_authority[0].disposition, "RetryAfterStateChange");

    let completed_handoff = rows_for("CANISTER_POOL_HANDOFF_ALREADY_COMPLETE");
    let reserved_store_cycles = rows_for("ROOT_STORE_DELETION_RESERVED_CYCLES_PRESENT");
    assert_eq!(completed_handoff.len(), 1);
    assert_eq!(reserved_store_cycles.len(), 1);
    assert_eq!(completed_handoff[0].remediation, "terminal_receipt_lookup");
    assert_eq!(completed_handoff[0].disposition, "DoNotRetry");
    assert_eq!(reserved_store_cycles[0].remediation, "state_reconciliation");
    assert_ne!(
        completed_handoff[0].handling_key,
        reserved_store_cycles[0].handling_key,
    );

    let cooldown = rows_for("RPC_CYCLES_FUNDING_COOLDOWN_ACTIVE");
    assert_eq!(cooldown.len(), 1);
    assert_eq!(cooldown[0].class, "ResourceExhausted");
    assert_eq!(cooldown[0].disposition, "RetryAfterStateChange");
    assert_eq!(cooldown[0].remediation, "state_progression");

    for identity in [
        "GROUP_PROVISIONING_CLAIM_COMPLETE",
        "GROUP_PROVISIONING_INSTALL_COMPLETE",
        "GROUP_PROVISIONING_REGISTRY_COMMIT_COMPLETE",
        "GROUP_PROVISIONING_RESERVATION_COMPLETE",
    ] {
        let complete_phase = rows_for(identity);
        assert_eq!(
            complete_phase.len(),
            1,
            "phase coverage drifted: {identity}"
        );
        assert_eq!(complete_phase[0].remediation, "phase_advance");
    }

    let reservation_time = rows_for("ROOT_DRAINING_RESERVATION_TIME_INVALID");
    assert_eq!(reservation_time.len(), 2);
    assert!(
        reservation_time
            .iter()
            .any(|row| row.exposure == "safe public identity")
    );
    assert!(reservation_time.iter().any(|row| {
        row.exposure == "internal; projected before return"
            && row.projection == "COMPONENT_REGISTRY_STATE_INVALID"
    }));
}

#[test]
fn projection_catalogue_is_complete_and_source_bound() {
    let root = inventory_root();
    let catalogue = projection_catalogue(&root);
    let observations = projection_observations(&root);

    assert_eq!(catalogue.len(), 31, "projection catalogue count drifted");
    assert_eq!(
        observations.len(),
        39,
        "projection observation count drifted"
    );
    for (projection, row) in catalogue {
        assert!(!row.class.is_empty(), "missing class for {projection}");
        assert!(
            !row.disposition.is_empty(),
            "missing disposition for {projection}"
        );
        assert!(
            !row.observation.is_empty(),
            "missing observation for {projection}"
        );
        assert!(!row.summary.is_empty(), "missing summary for {projection}");
        assert!(!row.action.is_empty(), "missing action for {projection}");
    }
}

#[test]
fn reviewed_compression_register_matches_the_complete_proposal() {
    let root = inventory_root();
    let register = render_compression_register(&root);

    if std::env::var_os("CANIC_UPDATE_DIAGNOSTIC_PROPOSAL").is_some() {
        update_compression_register_in_proposal(&register);
    }

    assert_eq!(
        compression_register_from_proposal(),
        register,
        "reviewed compression register drifted from its complete guarded map"
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the semantic evidence guard compares every qualification debt independently"
)]
fn semantic_action_evidence_is_qualified_and_classifiable() {
    let root = inventory_root();
    let materialized = materialized_inventory_tokens(&root);
    let projections = projection_tokens(&root);
    let exact = materialized
        .difference(&projections)
        .cloned()
        .collect::<BTreeSet<_>>();
    let evidence = compression_evidence(&root, &exact, &projections);
    let observations = source_observations(&root, &exact);
    let mut producer_exposures = BTreeMap::<(String, String), BTreeSet<ObservationExposure>>::new();
    let mut producer_actions = BTreeMap::<(String, String), BTreeSet<&'static str>>::new();
    let mut producer_pairs = BTreeSet::<(String, String)>::new();
    let mut observation_sources = BTreeSet::<String>::new();
    for observation in &observations {
        observation_sources.insert(observation.source.clone());
        let action = observation.action.as_deref().and_then(action_remediation);
        for producer in &observation.producers {
            let key = (observation.identity.clone(), producer.clone());
            producer_pairs.insert(key.clone());
            if let Some(action) = action {
                producer_actions.entry(key).or_default().insert(action);
            }
        }
        if observation.exposure == ObservationExposure::Unspecified {
            continue;
        }
        for producer in &observation.producers {
            producer_exposures
                .entry((observation.identity.clone(), producer.clone()))
                .or_default()
                .insert(observation.exposure.clone());
        }
    }
    let has_conflicting_producer_exposures = producer_exposures
        .values()
        .any(|exposures| exposures.len() > 1);
    let has_conflicting_producer_actions =
        producer_actions.values().any(|actions| actions.len() > 1);
    let mut identity_exposures = BTreeMap::<String, BTreeSet<ObservationExposure>>::new();
    for observation in &observations {
        if observation.exposure != ObservationExposure::Unspecified {
            identity_exposures
                .entry(observation.identity.clone())
                .or_default()
                .insert(observation.exposure.clone());
        }
    }
    let mut identity_actions = BTreeMap::<String, BTreeSet<&'static str>>::new();
    for observation in &observations {
        if let Some(action) = observation.action.as_deref().and_then(action_remediation) {
            identity_actions
                .entry(observation.identity.clone())
                .or_default()
                .insert(action);
        }
    }
    let producer_exposure_debt = producer_pairs
        .iter()
        .filter(|pair| !producer_exposures.contains_key(*pair))
        .cloned()
        .collect::<Vec<_>>();
    let producer_action_debt = producer_pairs
        .iter()
        .filter(|pair| !producer_actions.contains_key(*pair))
        .cloned()
        .collect::<Vec<_>>();
    let unresolved_exposure_debt = producer_exposure_debt
        .iter()
        .filter(|(identity, _)| identity_exposures.get(identity).map(BTreeSet::len) != Some(1))
        .count();
    let unresolved_action_debt = producer_action_debt
        .iter()
        .filter(|(identity, _)| identity_actions.get(identity).map(BTreeSet::len) != Some(1))
        .count();
    let unspecified_observations = observations
        .iter()
        .filter(|observation| observation.exposure == ObservationExposure::Unspecified)
        .count();
    let missing = evidence
        .iter()
        .filter(|(_, row)| row.actions.is_empty())
        .count();
    let multiple = evidence
        .iter()
        .filter(|(_, row)| row.actions.len() > 1)
        .count();
    let unique = evidence
        .values()
        .flat_map(|row| row.actions.iter())
        .collect::<BTreeSet<_>>();
    let has_unclassified = unique
        .iter()
        .any(|action| action_remediation(action).is_none());
    let conflicting_categories = evidence
        .iter()
        .filter_map(|(identity, row)| {
            let categories = row
                .actions
                .iter()
                .filter_map(|action| action_remediation(action))
                .collect::<BTreeSet<_>>();
            (categories.len() > 1).then_some((identity, categories, &row.actions))
        })
        .count();
    let semantic_mismatches = evidence
        .iter()
        .filter_map(|(identity, row)| {
            let categories = row
                .actions
                .iter()
                .filter_map(|action| action_remediation(action))
                .collect::<BTreeSet<_>>();
            let category = (categories.len() == 1)
                .then(|| categories.iter().next().copied())
                .flatten()?;
            let subject = compression_subject(identity);
            let condition = compression_condition(identity);
            let masked = !row.projections.is_empty();
            let disposition = compression_disposition(identity, subject, condition, masked);
            let current = compression_remediation(subject, condition, disposition, masked);
            (category != current).then_some((identity, current, category, &row.actions))
        })
        .count();
    let mixed_exposure = evidence
        .iter()
        .filter(|(_, row)| row.saw_public_self && !row.projections.is_empty())
        .count();
    let action_aware_groups = evidence
        .iter()
        .map(|(identity, row)| {
            let subject = compression_subject(identity);
            let condition = compression_condition(identity);
            let masked = !row.projections.is_empty();
            let derived_disposition = compression_disposition(identity, subject, condition, masked);
            let derived_remediation =
                compression_remediation(subject, condition, derived_disposition, masked);
            let categories = row
                .actions
                .iter()
                .filter_map(|action| action_remediation(action))
                .collect::<BTreeSet<_>>();
            let remediation = if masked {
                "state_reconciliation"
            } else if categories.len() == 1 {
                categories
                    .iter()
                    .next()
                    .copied()
                    .expect("one action category should exist")
            } else {
                derived_remediation
            };
            let disposition = match remediation {
                "exact_replay" => "exact_retry",
                "authority_refresh" | "capacity_relief" | "credential_renewal"
                | "phase_advance" | "reinstall" | "state_progression" => "retry_after_state_change",
                "effect_recovery" => "bounded_retry",
                "implementation_correction" | "state_reconciliation" | "manual_intervention" => {
                    "reconcile"
                }
                "request_correction" | "terminal_receipt_lookup" => "do_not_retry",
                other => panic!("unexpected action remediation: {other}"),
            };
            let class = match remediation {
                "capacity_relief" => "resource_exhausted",
                "state_progression" => "unavailable",
                "exact_replay" => "conflict",
                _ => compression_class(
                    compression_origin(identity, subject),
                    subject,
                    condition,
                    masked,
                ),
            };
            (
                compression_origin(identity, subject),
                subject,
                condition,
                class,
                disposition,
                remediation,
                row.projections.iter().next().cloned(),
            )
        })
        .collect::<BTreeSet<_>>();

    assert_eq!(evidence.len(), 2_864);
    assert_eq!(observations.len(), 3_351);
    assert_eq!(observation_sources.len(), 2_700);
    assert_eq!(producer_pairs.len(), 3_898);
    assert_eq!(unspecified_observations, 476);
    assert_eq!(producer_exposure_debt.len(), 527);
    assert_eq!(unresolved_exposure_debt, 433);
    assert!(!has_conflicting_producer_exposures);
    assert_eq!(producer_action_debt.len(), 493);
    assert_eq!(unresolved_action_debt, 424);
    assert!(!has_conflicting_producer_actions);
    assert_eq!(missing, 404);
    assert_eq!(multiple, 276);
    assert_eq!(unique.len(), 1_986);
    assert!(!has_unclassified);
    assert_eq!(conflicting_categories, 91);
    assert_eq!(semantic_mismatches, 1_182);
    assert_eq!(mixed_exposure, 105);
    assert_eq!(action_aware_groups.len(), 668);
}
