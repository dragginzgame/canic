//! Versioned, passive inspection data. Metadata grants no row, write or migration authority.
//! Each reply is one bounded observation, not a database snapshot. There is no pagination.
use ic_wire::{List, Text};
use serde::{Deserialize, Serialize};

/// Inspection v1, independent of the database's durable format.
pub const INSPECT: u64 = ic_wire::method_id("canic.db.inspect.v1");
/// Logical names, never physical storage handles.
pub type Name = Text<64>;
/// One complete request; entity selection binds the catalogue's executable schema.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Request {
    /// Complete exposed entity catalogue, at most 64 names.
    Catalogue,
    /// One exposed entity and its reachable named definitions.
    Entity {
        /// Expected executable schema identity from an earlier observation.
        schema: [u8; 32],
        /// Logical entity name.
        name: Name,
        /// Request maintained cardinality only when application policy permits it.
        count: bool,
    },
}
/// Ordinary inspection refusal. Never silently truncate a complete response.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Refusal {
    /// Requested schema no longer matches the executable.
    SchemaChanged,
    /// Entity is absent or not exposed by application policy.
    UnknownEntity,
    /// Catalogue, type graph, name or reply limits exceeded.
    Limit,
    /// The compiled projection cannot represent the supplied schema.
    Unsupported,
}
/// Stored admission, separate from the executable schema identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Admission {
    /// Ordinary database open succeeded; this is its readable incarnation.
    Readable {
        /// Database incarnation, not a schema hash or row revision.
        incarnation: [u8; 16],
    },
    /// Ordinary reads are fenced by maintenance; no incarnation was read.
    Maintenance,
    /// The executable cannot admit this stored layout; bytes remain untouched.
    Incompatible,
    /// Missing, corrupt or unavailable state; no initialization or repair is implied.
    Unavailable,
}
/// Maintained row metadata, never a hidden row scan.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Cardinality {
    /// Not requested or withheld by application policy.
    NotExposed,
    /// Ordinary admission and application permission allowed this observation.
    Available(u64),
    /// Requested, but ordinary admission refused or metadata could not be read.
    Unavailable,
}
/// Typed value semantics. Named links stay shallow, including recursive graphs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Kind {
    /// Exact unsigned integer, in bits.
    Unsigned(u16),
    /// Exact signed integer, in bits.
    Signed(u16),
    /// Finite IEEE floating point, in bits.
    Float(u16),
    /// Boolean.
    Bool,
    /// IC principal.
    Principal,
    /// Account identity.
    Account,
    /// Opaque 32-byte subaccount.
    Subaccount,
    /// ULID.
    Ulid,
    /// Gregorian epoch days.
    Date,
    /// Signed Unix milliseconds.
    Timestamp,
    /// Nonnegative milliseconds.
    Duration,
    /// Exact decimal with declared fractional scale.
    Decimal(u32),
    /// UTF-8 text byte ceiling.
    Text(u32),
    /// Binary byte ceiling.
    Blob(u32),
    /// Reference to a definition in this response, not inline expansion.
    Named(Name),
    /// Ordered bounded elements.
    List {
        /// Element shape.
        item: Box<Kind>,
        /// Maximum admitted elements.
        max_items: u32,
    },
}
/// Value type plus explicit presence semantics.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Value {
    /// Type and its authored bounds.
    pub kind: Kind,
    /// Whether absence is distinct from every present value.
    pub optional: bool,
}
/// Reference target and supporting index are distinct facts, not access grants.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Reference {
    /// Target logical entity.
    pub target: Name,
    /// Reverse-lookup index on the referencing entity.
    pub index: Name,
}
/// One logical field. This is inspection, not a write-input schema.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Field {
    /// Logical identity.
    pub name: Name,
    /// Stored value shape.
    pub value: Value,
    /// Optional cross-entity relationship.
    pub reference: Option<Reference>,
}
/// One named index, with ordered key components.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Index {
    /// Logical index identity.
    pub name: Name,
    /// Ordered component fields.
    pub fields: List<Name, 16>,
    /// Complete keys must be unique.
    pub unique: bool,
    /// Optional components must all be present for membership.
    pub present_only: bool,
}
/// One named enum alternative.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Variant {
    /// Logical alternative name.
    pub name: Name,
    /// Unit or explicitly typed payload.
    pub payload: Option<Value>,
}
/// Named graph node, with finite shallow edges.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Body {
    /// Named record fields.
    Record(List<Field, 64>),
    /// Unit and payload alternatives.
    Enum(List<Variant, 64>),
    /// Ordered set type and size ceiling.
    Set {
        /// Element shape.
        item: Kind,
        /// Maximum elements.
        max_items: u32,
    },
    /// Ordered map type and size ceiling.
    Map {
        /// Key shape.
        key: Kind,
        /// Value shape.
        value: Value,
        /// Maximum entries.
        max_items: u32,
    },
}
/// One named definition; names are unique in a complete projection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Definition {
    /// Logical identity.
    pub name: Name,
    /// Definition, without recursively expanding named links.
    pub body: Body,
}
/// Complete bounded selected entity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Entity {
    /// Logical identity.
    pub name: Name,
    /// Primary-key field name.
    pub primary_key: Name,
    /// Declared fields.
    pub fields: List<Field, 64>,
    /// Declared indexes.
    pub indexes: List<Index, 64>,
    /// Reachable named types, each emitted once, at most 64 definitions.
    pub types: List<Definition, 64>,
    /// Explicitly policy-selected row-count observation.
    pub cardinality: Cardinality,
}
/// Exact selection, with no truncated-success or continuation form.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Selection {
    /// Complete exposed logical catalogue.
    Catalogue(List<Name, 64>),
    /// Complete selected entity and reachable types.
    Entity(Entity),
}
/// One observation. Schema identity does not attest storage admission or integrity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Response {
    /// Executable compiled-schema identity.
    pub schema: [u8; 32],
    /// Independently obtained ordinary storage admission.
    pub admission: Admission,
    /// Application-selected metadata.
    pub selection: Selection,
}
/// Shared reply, usable by native clients without importing IC DB.
pub type Reply = Result<Response, Refusal>;
