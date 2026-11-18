//! Check source declarations only for packages that adopt Canic's layout.
use crate::{Error, Result, supplier};
use serde::Deserialize;
use std::path::{Component, Path, PathBuf};
use syn::visit::Visit;

#[derive(Deserialize)]
pub(super) struct Package {
    pub id: String,
    name: String,
    manifest_path: PathBuf,
    metadata: serde_json::Value,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Layout {
    endpoints: Option<PathBuf>,
    schema: Option<PathBuf>,
    lifecycle: Option<PathBuf>,
}

impl Package {
    pub fn validate(&self) -> Result<()> {
        let Some(value) = self.metadata.pointer("/canic/layout") else {
            return Ok(());
        };
        let layout: Layout = serde_json::from_value(value.clone()).map_err(supplier)?;
        let roots = [
            (
                "endpoints",
                layout
                    .endpoints
                    .as_deref()
                    .unwrap_or(Path::new("src/endpoints")),
            ),
            (
                "schema",
                layout.schema.as_deref().unwrap_or(Path::new("src/schema")),
            ),
            (
                "lifecycle",
                layout
                    .lifecycle
                    .as_deref()
                    .unwrap_or(Path::new("src/lifecycle.rs")),
            ),
        ];
        for (key, root) in &roots {
            if !root.starts_with("src")
                || root == &Path::new("src")
                || !root.components().all(|c| matches!(c, Component::Normal(_)))
            {
                return Err(Error::Invalid(format!(
                    "{}: package.metadata.canic.layout.{key} must name a path inside src",
                    self.name
                )));
            }
        }
        let base = self
            .manifest_path
            .parent()
            .ok_or_else(|| Error::Invalid("Cargo manifest has no parent".into()))?;
        let mut pending = vec![PathBuf::from("src")];
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(base.join(&dir)).map_err(supplier)? {
                let entry = entry.map_err(supplier)?;
                let path = dir.join(entry.file_name());
                let kind = entry.file_type().map_err(supplier)?;
                if kind.is_dir() {
                    pending.push(path);
                } else if kind.is_symlink() {
                    return Err(Error::Invalid(format!(
                        "{}: cannot validate symlink {}",
                        self.name,
                        path.display()
                    )));
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    let bytes = crate::store::read(&base.join(&path), 1024 * 1024)?;
                    let syntax = syn::parse_file(std::str::from_utf8(&bytes).map_err(supplier)?)
                        .map_err(supplier)?;
                    let mut found = Declarations::default();
                    found.visit_file(&syntax);
                    for ((key, root), present) in roots.iter().zip(found.0) {
                        if present && !path.starts_with(root) {
                            return Err(Error::Invalid(format!(
                                "{}: {} contains {key} declarations; expected {} (override package.metadata.canic.layout.{key} in this crate's Cargo.toml)",
                                self.name,
                                path.display(),
                                root.display()
                            )));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Default)]
struct Declarations([bool; 3]);
impl<'ast> Visit<'ast> for Declarations {
    fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
        if let Some(segment) = attr.path().segments.last() {
            match segment.ident.to_string().as_str() {
                "query" | "update" => self.0[0] = true,
                "entity" => self.0[1] = true,
                "init" | "pre_upgrade" | "post_upgrade" => self.0[2] = true,
                _ => {}
            }
        }
    }
    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        if let Some(segment) = item.mac.path.segments.last() {
            let name = segment.ident.to_string();
            if name == "mount_auth_lifecycle" {
                self.0[2] = true;
            }
            if name.starts_with("mount_")
                && (name.contains("endpoint")
                    || name == "mount_wire_method"
                    || name == "mount_auth_session")
            {
                self.0[0] = true;
            }
        }
    }
}
