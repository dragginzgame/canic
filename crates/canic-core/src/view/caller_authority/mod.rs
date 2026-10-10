//! Borrowed receiver authority used by pure admission decisions.

use canic_contracts::ids::{CallerInstallation, CallerReceiverAuthority};

/// Borrowed evidence for one local admission; Component fencing is an indexed lookup.

pub struct CallerAdmissionView<'a> {
    pub receiver: &'a CallerReceiverAuthority,
    pub generation: u64,
    pub receiver_open: bool,
    pub source: Option<&'a CallerInstallation>,
    pub source_open: bool,
    pub component_fenced: bool,
}
