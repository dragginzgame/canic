use canic_host::evidence_envelope::command_path_for_root;
use std::path::Path;

pub fn push_optional_path_arg(
    args: &mut Vec<String>,
    redactions: &mut Vec<String>,
    flag: &str,
    path: Option<&Path>,
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

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure absent optional evidence inputs do not alter command provenance.
    #[test]
    fn absent_path_does_not_push_args_or_redactions() {
        let mut args = vec!["canic".to_string()];
        let mut redactions = Vec::new();

        push_optional_path_arg(
            &mut args,
            &mut redactions,
            "--evidence",
            None,
            Path::new("/repo"),
        );

        assert_eq!(args, ["canic"]);
        assert!(redactions.is_empty());
    }

    // Ensure evidence paths outside the config root are redacted in argv.
    #[test]
    fn outside_root_path_pushes_redaction() {
        let mut args = vec!["canic".to_string()];
        let mut redactions = Vec::new();

        push_optional_path_arg(
            &mut args,
            &mut redactions,
            "--evidence",
            Some(Path::new("/tmp/evidence.json")),
            Path::new("/repo"),
        );

        assert_eq!(args.len(), 3);
        assert_eq!(args[1], "--evidence");
        assert!(args[2].starts_with("<redacted:"));
        assert_eq!(redactions, ["--evidence absolute path outside config root"]);
    }
}
