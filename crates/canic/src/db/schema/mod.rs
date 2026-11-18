//! Bounded projection of admitted compiled schemas. No database open, storage read,
//! row scan, permission decision, lifecycle action or mutation occurs here.

#[cfg(test)]
mod tests;

use super::inspection::{
    Admission, Body, Cardinality, Definition, Entity, Field, INSPECT, Index, Kind, Name, Reference,
    Refusal, Reply, Request, Response, Selection, Value, Variant,
};
use ic_db::{FieldSchema, FieldType, Schema, TypeBody, TypeSchema};
use ic_wire::List;
use std::collections::BTreeMap;

fn name(value: &str) -> Result<Name, Refusal> {
    Name::new(value.into()).map_err(|_| Refusal::Limit)
}

fn list<T, const N: usize>(values: Vec<T>) -> Result<List<T, N>, Refusal> {
    List::new(values).map_err(|_| Refusal::Limit)
}
struct Projection {
    reached: BTreeMap<&'static str, &'static TypeSchema>,
}
impl Projection {
    fn kind(&mut self, value: FieldType) -> Result<Kind, Refusal> {
        Ok(match value {
            FieldType::Nat8 => Kind::Unsigned(8),
            FieldType::Nat16 => Kind::Unsigned(16),
            FieldType::Nat32 => Kind::Unsigned(32),
            FieldType::Nat64 => Kind::Unsigned(64),
            FieldType::Nat128 => Kind::Unsigned(128),
            FieldType::U256 => Kind::Unsigned(256),
            FieldType::Int8 => Kind::Signed(8),
            FieldType::Int16 => Kind::Signed(16),
            FieldType::Int32 => Kind::Signed(32),
            FieldType::Int64 => Kind::Signed(64),
            FieldType::Int128 => Kind::Signed(128),
            FieldType::Float32 => Kind::Float(32),
            FieldType::Float64 => Kind::Float(64),
            FieldType::Bool => Kind::Bool,
            FieldType::Principal => Kind::Principal,
            FieldType::Account => Kind::Account,
            FieldType::Subaccount => Kind::Subaccount,
            FieldType::Ulid => Kind::Ulid,
            FieldType::Date => Kind::Date,
            FieldType::Timestamp => Kind::Timestamp,
            FieldType::Duration => Kind::Duration,
            FieldType::Decimal { scale } => Kind::Decimal(scale),
            FieldType::Text { max_bytes } => Kind::Text(max_bytes),
            FieldType::Blob { max_bytes } => Kind::Blob(max_bytes),
            FieldType::List { item, max_items } => Kind::List {
                item: Box::new(self.kind(*item)?),
                max_items,
            },
            FieldType::Named(link) => {
                if !self.reached.contains_key(link.name) {
                    if self.reached.len() == 64 {
                        return Err(Refusal::Limit);
                    }
                    self.reached.insert(link.name, (link.definition)());
                }
                Kind::Named(name(link.name)?)
            }
        })
    }

    fn value(&mut self, value: FieldType, optional: bool) -> Result<Value, Refusal> {
        Ok(Value {
            kind: self.kind(value)?,
            optional,
        })
    }

    fn fields(
        &mut self,
        fields: &[FieldSchema],
        indexes: &[ic_db::IndexSchema],
    ) -> Result<List<Field, 64>, Refusal> {
        if fields.len() > 64 {
            return Err(Refusal::Limit);
        }
        list(
            fields
                .iter()
                .map(|field| {
                    Ok(Field {
                        name: name(field.name)?,
                        value: self.value(field.value, field.optional)?,
                        reference: field
                            .reference
                            .map(|reference| {
                                Ok(Reference {
                                    target: name(reference.entity_name)?,
                                    index: name(
                                        indexes
                                            .iter()
                                            .find(|i| i.id == reference.index)
                                            .ok_or(Refusal::Unsupported)?
                                            .name,
                                    )?,
                                })
                            })
                            .transpose()?,
                    })
                })
                .collect::<Result<_, Refusal>>()?,
        )
    }

    fn body(&mut self, value: &TypeBody) -> Result<Body, Refusal> {
        Ok(match value {
            TypeBody::Wrapped(_) => return Err(Refusal::Unsupported),
            TypeBody::Record(fields) => Body::Record(self.fields(fields, &[])?),
            TypeBody::Enum(variants) => {
                if variants.len() > 64 {
                    return Err(Refusal::Limit);
                }
                Body::Enum(list(
                    variants
                        .iter()
                        .map(|v| {
                            Ok(Variant {
                                name: name(v.name)?,
                                payload: v
                                    .payload
                                    .map(|p| self.value(p.value, p.optional))
                                    .transpose()?,
                            })
                        })
                        .collect::<Result<_, Refusal>>()?,
                )?)
            }
            TypeBody::Set { item, max_items } => Body::Set {
                item: self.kind(*item)?,
                max_items: *max_items,
            },
            TypeBody::Map {
                key,
                value,
                max_items,
            } => Body::Map {
                key: self.kind(*key)?,
                value: self.value(value.value, value.optional)?,
                max_items: *max_items,
            },
        })
    }

    fn definitions(&mut self) -> Result<List<Definition, 64>, Refusal> {
        let mut done = BTreeMap::new();
        while let Some((&id, &definition)) =
            self.reached.iter().find(|(id, _)| !done.contains_key(*id))
        {
            let body = self.body(&definition.body)?;
            done.insert(
                id,
                Definition {
                    name: name(id)?,
                    body,
                },
            );
        }
        list(done.into_values().collect())
    }
}

/// Project the application's exposed compiled schema, without opening its database.
/// The caller must independently authorize inspection and obtain admission/count
/// from the real database owner. No catalogue continuation or row scan is supplied.
/// # Errors
/// Refuses schema substitution, absent entities and bounded complete projections.
pub fn inspect(
    schema: &Schema,
    request: &Request,
    admission: Admission,
    cardinality: Cardinality,
) -> Reply {
    let selection = match request {
        Request::Catalogue => {
            if schema.entities().len() > 64 {
                return Err(Refusal::Limit);
            }
            Selection::Catalogue(list(
                schema
                    .entities()
                    .iter()
                    .map(|e| name(e.name))
                    .collect::<Result<_, _>>()?,
            )?)
        }
        Request::Entity {
            schema: expected,
            name: selected,
            count,
        } => {
            if expected != &schema.fingerprint() {
                return Err(Refusal::SchemaChanged);
            }
            let entity = schema
                .entities()
                .iter()
                .find(|e| e.name == selected.as_str())
                .ok_or(Refusal::UnknownEntity)?;
            if entity.indexes.len() > 64 {
                return Err(Refusal::Limit);
            }
            let mut projection = Projection {
                reached: BTreeMap::new(),
            };
            let fields = projection.fields(entity.fields, entity.indexes)?;
            let indexes = list(
                entity
                    .indexes
                    .iter()
                    .map(|i| {
                        if i.fields.len() > 16 {
                            return Err(Refusal::Limit);
                        }
                        Ok(Index {
                            name: name(i.name)?,
                            fields: list(
                                i.fields.iter().map(|f| name(f)).collect::<Result<_, _>>()?,
                            )?,
                            unique: i.unique,
                            present_only: i.present_only,
                        })
                    })
                    .collect::<Result<_, _>>()?,
            )?;
            let cardinality = if !count {
                Cardinality::NotExposed
            } else if matches!(admission, Admission::Readable { .. }) {
                cardinality
            } else {
                Cardinality::Unavailable
            };
            Selection::Entity(Entity {
                name: name(entity.name)?,
                primary_key: name(entity.primary_key)?,
                fields,
                indexes,
                types: projection.definitions()?,
                cardinality,
            })
        }
    };
    let response = Response {
        schema: schema.fingerprint(),
        admission,
        selection,
    };
    ic_wire::encode(
        &Ok::<_, Refusal>(&response),
        ic_wire::Encoding::Frame {
            method: INSPECT,
            direction: ic_wire::Direction::Response,
        },
        super::RPC_LIMITS,
    )
    .map_err(|_| Refusal::Limit)?;
    Ok(response)
}
