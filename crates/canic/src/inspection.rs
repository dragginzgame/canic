//! Native metadata for methods linked from an inspected actor.
use ic_wire::typescript::{SchemaLimits, SchemaRegistry, Wire};
use serde::Serialize;
use serde_json::Value;
use std::{
    any::{TypeId, type_name},
    collections::BTreeMap,
    marker::PhantomData,
};

#[doc(hidden)]
pub use inventory;

type Graph = Result<Value, String>;

/// Qualified layouts for Serde types whose representation is owned by the consumer.
#[derive(Default)]
pub struct Schemas(BTreeMap<TypeId, fn() -> Graph>);

impl Schemas {
    /// Use a separately qualified Wire layout for this exact Rust root.
    pub fn describe_as<T: 'static, Layout: Wire>(&mut self) {
        self.0.insert(TypeId::of::<T>(), graph::<Layout>);
    }
}

fn graph<T: Wire>() -> Graph {
    let mut registry =
        SchemaRegistry::new(SchemaLimits::default()).map_err(|e| format!("{e:?}"))?;
    registry
        .register::<T>("Root")
        .map_err(|e| format!("{e:?}"))?;
    let json = registry
        .finish()
        .and_then(|g| g.metadata_json())
        .map_err(|e| format!("{e:?}"))?;
    serde_json::from_str(&json).map_err(|e| format!("{e:?}"))
}

/// One Rust root and its admitted Wire graph rooted at `Root`.
#[derive(Serialize)]
pub struct Root {
    /// Concrete Rust type at the endpoint boundary.
    pub rust: &'static str,
    /// Wire metadata, including reachable named definitions.
    pub schema: Value,
}

#[doc(hidden)]
pub struct Type<T>(pub PhantomData<T>);

#[doc(hidden)]
pub trait Describe {
    fn describe(self, schemas: &Schemas) -> Result<Root, String>;
}

impl<T: 'static> Describe for &Type<T> {
    fn describe(self, schemas: &Schemas) -> Result<Root, String> {
        let rust = type_name::<T>();
        let layout = schemas
            .0
            .get(&TypeId::of::<T>())
            .ok_or_else(|| format!("no qualified schema for {rust}"))?;
        Ok(Root {
            rust,
            schema: layout()?,
        })
    }
}

impl<T: Wire + 'static> Describe for &&Type<T> {
    fn describe(self, _: &Schemas) -> Result<Root, String> {
        Ok(Root {
            rust: type_name::<T>(),
            schema: graph::<T>()?,
        })
    }
}

// Auth's published types implement the schema trait from its own pinned Wire version.
#[cfg(feature = "auth")]
#[doc(hidden)]
pub fn auth_root<T: crate::auth::protocol::wire::typescript::Wire>(
    _: &Schemas,
) -> Result<Root, String> {
    use crate::auth::protocol::wire::typescript::{SchemaLimits, SchemaRegistry};
    let mut registry =
        SchemaRegistry::new(SchemaLimits::default()).map_err(|e| format!("{e:?}"))?;
    registry
        .register::<T>("Root")
        .map_err(|e| format!("{e:?}"))?;
    let json = registry
        .finish()
        .and_then(|g| g.metadata_json())
        .map_err(|e| format!("{e:?}"))?;
    Ok(Root {
        rust: type_name::<T>(),
        schema: serde_json::from_str(&json).map_err(|e| format!("{e:?}"))?,
    })
}

#[doc(hidden)]
pub struct Declaration {
    pub module: &'static str,
    pub name: &'static str,
    pub mode: &'static str,
    pub id: u64,
    pub request: fn(&Schemas) -> Result<Root, String>,
    pub reply: fn(&Schemas) -> Result<Root, String>,
}
inventory::collect!(Declaration);

/// A mounted method's effective ABI.
#[derive(Serialize)]
pub struct Method {
    /// Query or update.
    pub mode: &'static str,
    /// Routing ID in the Wire frame, serialized as hexadecimal to preserve all 64 bits.
    #[serde(serialize_with = "serialize_id")]
    pub id: u64,
    /// Decoded request.
    pub request: Root,
    /// Encoded response.
    pub reply: Root,
}

fn serialize_id<S: serde::Serializer>(id: &u64, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(&format_args!("0x{id:016x}"))
}

/// Inspect the linked actor by Rust crate name. The returned keys are the export names.
/// # Errors
/// Reports missing actors, duplicate exports, or a failed method and schema direction.
pub fn methods(actor: &str, schemas: &Schemas) -> Result<BTreeMap<&'static str, Method>, String> {
    collect(
        actor,
        schemas,
        inventory::iter::<Declaration>
            .into_iter()
            .filter(|d| d.module.split("::").next() == Some(actor)),
    )
}

/// Emit every mounted declaration linked into a single-actor inspection executable.
/// Link exactly one actor, including the same optional capabilities as its Wasm.
/// # Errors
/// Reports ambiguous exports or missing request/reply schemas.
pub fn print_manifest(schemas: &Schemas) -> Result<(), String> {
    let methods = collect(
        "linked actor",
        schemas,
        inventory::iter::<Declaration>.into_iter(),
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(&methods).map_err(|e| e.to_string())?
    );
    Ok(())
}

fn collect(
    actor: &str,
    schemas: &Schemas,
    declarations: impl Iterator<Item = &'static Declaration>,
) -> Result<BTreeMap<&'static str, Method>, String> {
    let mut methods = BTreeMap::new();
    for d in declarations {
        let root = |f: fn(&Schemas) -> Result<Root, String>, side| {
            f(schemas).map_err(|e| format!("{actor}.{} {side}: {e}", d.name))
        };
        let method = Method {
            mode: d.mode,
            id: d.id,
            request: root(d.request, "request")?,
            reply: root(d.reply, "reply")?,
        };
        if methods.insert(d.name, method).is_some() {
            return Err(format!("duplicate export {actor}.{}", d.name));
        }
    }
    if methods.is_empty() {
        return Err(format!("no inspected methods linked for {actor}"));
    }
    Ok(methods)
}
