//! Module: canister_protocol::contract
//!
//! Responsibility: parse and type-check Candid without printing parser diagnostics.
//! Does not own: source selection, network calls or storage.
//! Boundary: parsed contracts are inspected without adapting their wire shapes.

use ic_platform::candid::contract::TypeEnv;
use ic_platform::candid::contract::types::Type;

/// Parse and type-check a text contract without printing diagnostics to stderr.
///
/// Callers own error reporting. The parser's pretty loader writes directly to
/// stderr, bypassing both structured diagnostics and libtest's output capture.
pub fn parse_text(source: &str) -> ic_platform::candid_parser::Result<(TypeEnv, Option<Type>)> {
    let program = source.parse::<ic_platform::candid_parser::IDLProg>()?;
    let mut environment = TypeEnv::new();
    let actor = ic_platform::candid_parser::check_prog(&mut environment, &program)?;
    Ok((environment, actor))
}
