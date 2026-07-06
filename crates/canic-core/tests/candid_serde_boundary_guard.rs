// Category C - System-level artifact test (no embedded config).

use std::{
    fs,
    path::{Path, PathBuf},
};

#[test]
fn candid_types_use_supported_serde_rename_attributes() {
    let mut violations = Vec::new();

    for root in source_roots() {
        scan_rust_files(&root, &mut |path, contents| {
            inspect_candid_items(path, contents, &mut violations);
        });
    }

    assert!(
        violations.is_empty(),
        "Candid-bearing serde attribute boundary changed: {violations:?}"
    );
}

fn inspect_candid_items(path: &Path, contents: &str, violations: &mut Vec<String>) {
    let mut pending_attrs = Vec::<String>::new();
    let mut in_attr = false;
    let mut active_candid_item = None::<ActiveCandidItem>;

    for (index, line) in contents.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim();

        if let Some(item) = active_candid_item.as_mut() {
            if has_serde_alias_attr(trimmed) {
                violations.push(format!(
                    "{}:{line_number} uses serde(alias) inside CandidType item `{}`",
                    display(path),
                    item.name
                ));
            }

            item.observe_line(line);
            if item.is_closed() {
                active_candid_item = None;
            }
            continue;
        }

        if in_attr || trimmed.starts_with("#[") {
            in_attr = !trimmed.contains(']');
            pending_attrs.push(trimmed.to_string());
            continue;
        }

        if let Some(name) = item_name(trimmed) {
            let attrs = pending_attrs.join("\n");
            if attrs.contains("CandidType")
                && (attrs.contains("serde(rename_all") || attrs.contains("rename_all_fields"))
            {
                violations.push(format!(
                    "{}:{line_number} uses unsupported serde rename_all on CandidType item `{name}`",
                    display(path)
                ));
            }
            if attrs.contains("CandidType") && has_serde_alias_attr(&attrs) {
                violations.push(format!(
                    "{}:{line_number} uses serde(alias) on CandidType item `{name}`",
                    display(path)
                ));
            }
            if attrs.contains("CandidType") {
                let mut item = ActiveCandidItem::new(name);
                item.observe_line(line);
                if !item.is_closed() {
                    active_candid_item = Some(item);
                }
            }
            pending_attrs.clear();
            continue;
        }

        if !trimmed.is_empty() && !trimmed.starts_with("//") {
            pending_attrs.clear();
        }
    }
}

fn has_serde_alias_attr(source: &str) -> bool {
    source
        .lines()
        .any(|line| line.trim_start().starts_with("#[serde(") && line.contains("alias"))
}

struct ActiveCandidItem {
    name: String,
    brace_depth: usize,
    body_started: bool,
}

impl ActiveCandidItem {
    const fn new(name: String) -> Self {
        Self {
            name,
            brace_depth: 0,
            body_started: false,
        }
    }

    fn observe_line(&mut self, line: &str) {
        for ch in line.chars() {
            match ch {
                '{' => {
                    self.body_started = true;
                    self.brace_depth += 1;
                }
                '}' if self.body_started => {
                    self.brace_depth = self
                        .brace_depth
                        .checked_sub(1)
                        .expect("balanced Rust source braces");
                }
                _ => {}
            }
        }
    }

    const fn is_closed(&self) -> bool {
        self.body_started && self.brace_depth == 0
    }
}

fn item_name(trimmed: &str) -> Option<String> {
    let line = trimmed
        .strip_prefix("pub(crate) ")
        .or_else(|| trimmed.strip_prefix("pub(super) "))
        .or_else(|| trimmed.strip_prefix("pub "))
        .unwrap_or(trimmed);

    let keyword = if line.starts_with("enum ") {
        "enum "
    } else if line.starts_with("struct ") {
        "struct "
    } else {
        return None;
    };

    line.strip_prefix(keyword)
        .and_then(|rest| {
            rest.split(|ch: char| !ch.is_alphanumeric() && ch != '_')
                .next()
        })
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
}

fn source_roots() -> Vec<PathBuf> {
    let workspace = workspace_root();
    vec![
        workspace.join("crates/canic/src"),
        workspace.join("crates/canic-core/src"),
        workspace.join("crates/canic-control-plane/src"),
        workspace.join("crates/canic-wasm-store/src"),
    ]
}

fn scan_rust_files(root: &Path, visitor: &mut impl FnMut(&Path, &str)) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_rust_files(&path, visitor);
            continue;
        }

        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }

        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        visitor(&path, &contents);
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(PathBuf::from)
        .expect("workspace root")
}

fn display(path: &Path) -> String {
    path.strip_prefix(workspace_root())
        .unwrap_or(path)
        .display()
        .to_string()
}
