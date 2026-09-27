//! Module: release_artifacts::host_inputs
//!
//! Responsibility: key native artifact producers without separately compiled host test modules.
//! Boundary: production sources, embedded inputs and Cargo configuration remain byte-bound;
//! the complete Cargo snapshot still guards concurrent source changes.

#[cfg(test)]
mod tests;

use canic_core::cdk::utils::hash::{hex_bytes, sha256_hex};
use ic_testkit::artifacts::{ArtifactCacheSpec, ResolvedCargoBuildInputs};
use proc_macro2::{TokenStream, TokenTree};
use std::{
    collections::BTreeSet,
    fs, io,
    path::{Path, PathBuf},
};
use syn::{
    Item, Meta, Token,
    punctuated::Punctuated,
    visit::{self, Visit},
};

/// Cache identity and the conservative producer snapshot guarding acquisition and publication.
pub(in crate::pic::fleet_registry::baseline) struct FixtureArtifactCache {
    pub(in crate::pic::fleet_registry::baseline) spec: ArtifactCacheSpec,
    inputs: ResolvedCargoBuildInputs,
}

impl FixtureArtifactCache {
    pub(super) fn bind(
        spec: ArtifactCacheSpec,
        host: &Path,
        inputs: ResolvedCargoBuildInputs,
    ) -> Self {
        let excluded = standalone_test_sources(host).expect("classify standalone host tests");
        let mut spec = spec;
        for input in inputs.inputs() {
            spec = bind_path(
                spec,
                input.label(),
                input.path(),
                inputs.exclusions(),
                &excluded,
            )
            .expect("bind native artifact producer inputs");
        }
        Self { spec, inputs }
    }

    pub(in crate::pic::fleet_registry::baseline) fn require_unchanged(&self) {
        assert!(
            self.inputs
                .is_content_current()
                .expect("recheck native producer inputs"),
            "native artifact producer inputs changed during cache acquisition"
        );
    }
}

fn bind_path(
    mut spec: ArtifactCacheSpec,
    label: &Path,
    path: &Path,
    generated: &[PathBuf],
    tests: &BTreeSet<PathBuf>,
) -> io::Result<ArtifactCacheSpec> {
    if excluded_path(path, generated, tests) {
        return Ok(spec);
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.is_dir() {
        let mut children = fs::read_dir(path)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<io::Result<Vec<_>>>()?;
        children.sort();
        children.retain(|child| !excluded_path(child, generated, tests));
        // Directory presence matters too, including empty producer directories.
        let names = children
            .iter()
            .map(|child| hex_bytes(child.file_name().unwrap().as_encoded_bytes()))
            .collect::<Vec<_>>();
        spec = spec.with_identity_bytes(
            &input_label("native-directory", label),
            &serde_json::to_vec(&names).map_err(io::Error::other)?,
        );
        for child in children {
            spec = bind_path(
                spec,
                &label.join(child.file_name().unwrap()),
                &child,
                generated,
                tests,
            )?;
        }
    } else {
        // Symlinks are never classified as tests; the cache follows and guards their content.
        spec = spec.with_input(&input_label("native-source", label), path);
    }
    Ok(spec)
}

fn excluded_path(path: &Path, generated: &[PathBuf], tests: &BTreeSet<PathBuf>) -> bool {
    generated.iter().any(|root| path.starts_with(root)) || tests.contains(path)
}

fn input_label(kind: &str, path: &Path) -> String {
    format!("{kind}/{}", sha256_hex(path.as_os_str().as_encoded_bytes()))
}

fn standalone_test_sources(host: &Path) -> io::Result<BTreeSet<PathBuf>> {
    let mut sources = Vec::new();
    collect_rust_sources(&host.join("src"), &mut sources)?;
    let parsed = sources
        .iter()
        .map(|path| {
            let source = fs::read_to_string(path)?;
            syn::parse_file(&source)
                .map(|syntax| (path, syntax))
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
        })
        .collect::<io::Result<Vec<_>>>()?;
    let mut tests = BTreeSet::new();
    for (path, syntax) in &parsed {
        for item in &syntax.items {
            if let Item::Mod(module) = item
                && module.content.is_none()
                && requires_test(&module.attrs)
            {
                let base = module_directory(path);
                let name = module.ident.to_string();
                let leaf = base.join(format!("{name}.rs"));
                if leaf.is_file() && !leaf.is_symlink() {
                    tests.insert(leaf);
                }
                let directory = base.join(name);
                if directory.is_dir() && !directory.is_symlink() {
                    let mut nested = Vec::new();
                    collect_rust_sources(&directory, &mut nested)?;
                    tests.extend(nested);
                }
            }
        }
    }
    loop {
        let mut embedded = BTreeSet::new();
        for (path, syntax) in &parsed {
            if tests.contains(*path) {
                continue;
            }
            let mut includes = EmbeddedInputs {
                path,
                module_base: module_directory(path),
                embedded: &mut embedded,
                uncertain: false,
            };
            includes.visit_file(syntax);
            // Unresolved includes or alternate module paths could make a test file a producer.
            // Retain the entire package in that case instead of guessing a dependency closure.
            if includes.uncertain {
                return Ok(BTreeSet::new());
            }
        }
        let before = tests.len();
        for path in embedded {
            tests.remove(&path);
        }
        if tests.len() == before {
            break;
        }
    }
    Ok(tests)
}

fn collect_rust_sources(path: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let path = entry.path();
        if kind.is_dir() {
            collect_rust_sources(&path, files)?;
        } else if kind.is_file() && path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

fn module_directory(path: &Path) -> PathBuf {
    match path.file_stem().and_then(|name| name.to_str()) {
        Some("lib" | "main" | "mod") => path.parent().unwrap().to_path_buf(),
        _ => path.with_extension(""),
    }
}

fn requires_test(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("cfg")
            && attribute
                .parse_args::<Meta>()
                .is_ok_and(|meta| requires_test_predicate(&meta))
    })
}

fn requires_test_predicate(meta: &Meta) -> bool {
    match meta {
        Meta::Path(path) => path.is_ident("test"),
        Meta::List(list) => {
            let Ok(predicates) =
                list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
            else {
                return false;
            };
            if list.path.is_ident("all") {
                predicates.iter().any(requires_test_predicate)
            } else if list.path.is_ident("any") {
                !predicates.is_empty() && predicates.iter().all(requires_test_predicate)
            } else {
                false
            }
        }
        Meta::NameValue(_) => false,
    }
}

struct EmbeddedInputs<'a> {
    path: &'a Path,
    module_base: PathBuf,
    embedded: &'a mut BTreeSet<PathBuf>,
    uncertain: bool,
}

impl<'ast> Visit<'ast> for EmbeddedInputs<'_> {
    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        if requires_test(&module.attrs) {
            return;
        }
        if module
            .attrs
            .iter()
            .any(|attribute| attribute.path().is_ident("path"))
        {
            self.uncertain = true;
        }
        // A production declaration can point at a file also declared under cfg(test).
        if module.content.is_none() {
            let directory = self.module_base.join(module.ident.to_string());
            let leaf = directory.with_extension("rs");
            if leaf.is_file() {
                self.embedded.insert(leaf);
            }
            let entry = directory.join("mod.rs");
            if entry.is_file() {
                self.embedded.insert(entry);
            }
        }
        let parent = self.module_base.clone();
        self.module_base.push(module.ident.to_string());
        visit::visit_item_mod(self, module);
        self.module_base = parent;
    }

    fn visit_macro(&mut self, invocation: &'ast syn::Macro) {
        let include = invocation.path.segments.last().is_some_and(|segment| {
            matches!(
                segment.ident.to_string().as_str(),
                "include" | "include_str" | "include_bytes"
            )
        });
        if invocation.path.is_ident("include") {
            self.uncertain = true;
        } else if include {
            // Literal includes have a precise source-file-relative path. Generated paths
            // and include expansions retain the conservative whole-package identity.
            if let Ok(path) = syn::parse2::<syn::LitStr>(invocation.tokens.clone()) {
                match self
                    .path
                    .parent()
                    .unwrap()
                    .join(path.value())
                    .canonicalize()
                {
                    Ok(path) => {
                        self.embedded.insert(path);
                    }
                    Err(_) => self.uncertain = true,
                }
            } else {
                self.uncertain = true;
            }
        } else if contains_source_reference(invocation.tokens.clone()) {
            self.uncertain = true;
        }
    }
}

fn contains_source_reference(tokens: TokenStream) -> bool {
    tokens.into_iter().any(|token| match token {
        TokenTree::Ident(name) => matches!(
            name.to_string().as_str(),
            "mod" | "include" | "include_str" | "include_bytes"
        ),
        TokenTree::Group(group) => contains_source_reference(group.stream()),
        _ => false,
    })
}
