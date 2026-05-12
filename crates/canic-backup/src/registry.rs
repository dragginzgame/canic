///
/// RegistryEntry
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryEntry {
    pub pid: String,
    pub role: Option<String>,
    pub kind: Option<String>,
    pub parent_pid: Option<String>,
    pub module_hash: Option<String>,
}
