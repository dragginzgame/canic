use canic_host::evidence_envelope::command_path_for_root;
use std::path::{Path, PathBuf};

pub fn push_optional_path_arg(
    args: &mut Vec<String>,
    redactions: &mut Vec<String>,
    flag: &str,
    path: Option<&PathBuf>,
    root: &Path,
) {
    if let Some(path) = path {
        args.push(flag.to_string());
        let display_path = command_path_for_root(path, root);
        if display_path.starts_with("<redacted:") {
            redactions.push(format!("{flag} absolute path outside config root"));
        }
        args.push(display_path);
    }
}
