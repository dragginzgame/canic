//! Generate browser clients from the Wasm-checked application inventory.
use crate::{Error, Result, supplier};
use ic_wire::typescript::{self, Method, PayloadLimits, ServiceMethod, ValidatedRegistry};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fmt::Write as _,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
struct Root {
    schema: BTreeMap<String, Value>,
}

#[derive(Deserialize)]
struct Limits {
    bytes: u64,
    nodes: u64,
    depth: u64,
}

impl Limits {
    fn payload(&self) -> Result<PayloadLimits> {
        // The inventory counts the 11-byte Wire frame header in each direction.
        let bytes = self
            .bytes
            .checked_sub(11)
            .ok_or_else(|| Error::Invalid("frame limit below Wire header size".into()))?;
        Ok(PayloadLimits {
            bytes,
            nodes: self.nodes,
            depth: self.depth,
        })
    }
}

#[derive(Deserialize)]
struct Endpoint {
    id: String,
    mode: String,
    request: Root,
    reply: Root,
    request_limits: Limits,
    reply_limits: Limits,
}

impl Endpoint {
    fn codec(&mut self) -> Result<String> {
        let mut types = BTreeMap::new();
        for (alias, direction) in [
            ("Request", &mut self.request),
            ("Response", &mut self.reply),
        ] {
            let root = direction
                .schema
                .remove("Root")
                .ok_or_else(|| Error::Invalid("missing Wire schema root".into()))?;
            for (name, definition) in std::mem::take(&mut direction.schema)
                .into_iter()
                .chain([(alias.into(), root)])
            {
                if types
                    .insert(name.clone(), definition.clone())
                    .is_some_and(|old| old != definition)
                {
                    return Err(Error::Invalid(format!(
                        "conflicting Wire definition {name}"
                    )));
                }
            }
        }
        let method = Method {
            export: "codec",
            id: u64::from_str_radix(
                self.id
                    .strip_prefix("0x")
                    .ok_or_else(|| Error::Invalid("expected hexadecimal method ID".into()))?,
                16,
            )
            .map_err(supplier)?,
            request: "Request",
            response: "Response",
            request_limits: self.request_limits.payload()?,
            response_limits: self.reply_limits.payload()?,
        };
        ValidatedRegistry::from_metadata(&types, Default::default())
            .and_then(|registry| {
                registry.method_module(&[method], "./wire-runtime.js", &BTreeMap::new(), |report| {
                    eprint!("{report}")
                })
            })
            .map_err(|error| {
                Error::Invalid(format!("TypeScript codec generation failed: {error:?}"))
            })
    }
}

/// Generate one typed client factory per application actor in a new output directory.
/// The caller supplies an authenticated raw transport bound to the chosen canister.
/// # Errors
/// Returns build, inspection, schema generation or output publication errors.
pub fn export_wire(config: &Path, app: Option<&str>, out: &Path) -> Result<PathBuf> {
    let (_, manifest) = crate::wire_manifest(config, app)?;
    let inventory: BTreeMap<String, BTreeMap<String, Endpoint>> =
        serde_json::from_slice(&manifest).map_err(supplier)?;
    let mut files = BTreeMap::from([(
        PathBuf::from("wire-runtime.ts"),
        typescript::RUNTIME.as_bytes().to_vec(),
    )]);
    let registry = ValidatedRegistry::from_metadata(&BTreeMap::new(), Default::default())
        .map_err(|error| Error::Invalid(format!("TypeScript service registry: {error:?}")))?;
    let mut imports = String::new();
    let mut clients = String::from("export const clients = {\n");
    for (index, (actor, methods)) in inventory.into_iter().enumerate() {
        let mut mounted = Vec::new();
        for (name, mut endpoint) in methods {
            let module = format!("method-{}", files.len());
            let codec = endpoint
                .codec()
                .map_err(|error| Error::Invalid(format!("{actor}.{name}: {error}")))?;
            files.insert(format!("{module}.ts").into(), codec.into_bytes());
            mounted.push((name, endpoint.mode, format!("./{module}.js")));
        }
        let methods = mounted
            .iter()
            .map(|(name, mode, module)| ServiceMethod {
                binding: name,
                endpoint: name,
                mode,
                codec_export: "codec",
                codec_module: module,
            })
            .collect::<Vec<_>>();
        let service = registry.service_module(&methods).map_err(|error| {
            Error::Invalid(format!(
                "{actor}: TypeScript service generation failed: {error:?}"
            ))
        })?;
        files.insert(format!("service-{index}.ts").into(), service.into_bytes());
        writeln!(
            imports,
            "import {{ createService as service{index} }} from './service-{index}.js';"
        )
        .unwrap();
        if index == 0 {
            imports.push_str("export type { Transport } from './service-0.js';\n");
        }
        writeln!(clients, "  [{}]: service{index},", serde_json::json!(actor)).unwrap();
    }
    clients.push_str("};\n");
    imports.push_str(&clients);
    files.insert("index.ts".into(), imports.into_bytes());
    crate::starter::publish_files(out, &files, "wire-export")
}
