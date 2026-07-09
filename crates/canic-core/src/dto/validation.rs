use serde::Deserialize;

//
// ValidationReport
//

#[expect(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ValidationReport {
    pub ok: bool,
    pub registry_index_consistent: bool,
    pub unique_index_roles: bool,
    pub env_complete: bool,
    pub issues: Vec<ValidationIssue>,
}

//
// ValidationIssue
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ValidationIssue {
    pub code: String,
    pub message: String,
}
