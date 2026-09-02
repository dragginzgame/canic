//! Module: fleet_ensure::json
//!
//! Responsibility: encode current Fleet Ensure plan and report JSON projections.
//! Does not own: plan hashing, schema selection, or durable file replacement.
//! Boundary: every nested `u128` is bounded decimal text and report payloads stay external.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::model::{
    CanisterPlan, CurrentFleetProtocolAction, EnsureAction, FleetEnsurePlan, FleetEnsurePlanScope,
    FleetEnsureReport,
};
use std::fmt::Display;

use canic_core::cdk::utils::hash::sha256_hex;
use serde::{
    Serialize, Serializer,
    ser::{
        SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
        SerializeTupleStruct, SerializeTupleVariant,
    },
};
use serde_json::{Map, Value};

const STORE_CHUNK_OBJECT_DIRECTORY: &str = ".canic/fleet-ensure/objects/sha256";

/// Project one complete report without expanding content-addressed Store chunk bytes.
///
/// Each chunk retains a workspace-relative object path, its exact SHA-256 and its byte size.
pub fn report_json_value(report: &FleetEnsureReport) -> Result<Value, serde_json::Error> {
    let mut projection = Map::new();
    insert_serialized(
        &mut projection,
        "actual_conservation",
        &report.actual_conservation,
    )?;
    insert_serialized(&mut projection, "effects_applied", &report.effects_applied)?;
    projection.insert("plan".to_string(), plan_json_value(&report.plan)?);
    insert_serialized(&mut projection, "terminal", &report.terminal)?;
    Ok(Value::Object(projection))
}

fn plan_json_value(plan: &FleetEnsurePlan) -> Result<Value, serde_json::Error> {
    let mut projection = Map::new();
    projection.insert(
        "canisters".to_string(),
        Value::Array(
            plan.canisters
                .iter()
                .map(canister_json_value)
                .collect::<Result<Vec<_>, _>>()?,
        ),
    );
    insert_serialized(&mut projection, "conservation", &plan.conservation)?;
    insert_serialized(&mut projection, "desired_sha256", &plan.desired_sha256)?;
    insert_serialized(&mut projection, "environment", &plan.environment)?;
    insert_serialized(&mut projection, "fleet", &plan.fleet)?;
    insert_serialized(&mut projection, "operation_id", &plan.operation_id)?;
    insert_serialized(&mut projection, "plan_sha256", &plan.plan_sha256)?;
    insert_serialized(&mut projection, "planned_at_time", &plan.planned_at_time)?;
    projection.insert(
        "protocol_actions".to_string(),
        Value::Array(
            plan.protocol_actions
                .iter()
                .map(action_json_value)
                .collect::<Result<Vec<_>, _>>()?,
        ),
    );
    if let Some(authority) = &plan.root_start_authority {
        insert_serialized(&mut projection, "root_start_authority", authority)?;
    }
    if let Some(desired) = &plan.reviewed_desired {
        insert_serialized(&mut projection, "reviewed_desired", desired)?;
    }
    insert_serialized(&mut projection, "schema_version", &plan.schema_version)?;
    if plan.scope != FleetEnsurePlanScope::Full {
        insert_serialized(&mut projection, "scope", &plan.scope)?;
    }
    if let Some(operation_id) = &plan.terminal_inventory_operation_id {
        insert_serialized(
            &mut projection,
            "terminal_inventory_operation_id",
            operation_id,
        )?;
    }
    Ok(Value::Object(projection))
}

fn canister_json_value(canister: &CanisterPlan) -> Result<Value, serde_json::Error> {
    let mut projection = Map::new();
    projection.insert(
        "actions".to_string(),
        Value::Array(
            canister
                .actions
                .iter()
                .map(action_json_value)
                .collect::<Result<Vec<_>, _>>()?,
        ),
    );
    insert_serialized(&mut projection, "disposition", &canister.disposition)?;
    insert_serialized(&mut projection, "name", &canister.name)?;
    insert_serialized(
        &mut projection,
        "observed_cycles",
        &canister.observed_cycles,
    )?;
    insert_serialized(&mut projection, "principal", &canister.principal)?;
    Ok(Value::Object(projection))
}

fn action_json_value(action: &EnsureAction) -> Result<Value, serde_json::Error> {
    let EnsureAction::FleetProtocol {
        action: protocol_action,
        ..
    } = action
    else {
        return to_value(action);
    };
    let CurrentFleetProtocolAction::PublishStoreChunk { request } = protocol_action.as_ref() else {
        return to_value(action);
    };

    let bytes_sha256 = sha256_hex(&request.bytes);
    let bytes_size = u64::try_from(request.bytes.len()).unwrap_or(u64::MAX);
    let mut compact = action.clone();
    let EnsureAction::FleetProtocol {
        action: compact_protocol_action,
        ..
    } = &mut compact
    else {
        unreachable!("cloned Fleet protocol action retains its variant")
    };
    let CurrentFleetProtocolAction::PublishStoreChunk {
        request: compact_request,
    } = compact_protocol_action.as_mut()
    else {
        unreachable!("cloned Store publication retains its variant")
    };
    compact_request.bytes.clear();

    let mut projection = to_value(&compact)?;
    let projected_request = projection
        .get_mut("action")
        .and_then(|value| value.get_mut("request"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| json_shape_error("Store publication report projection is invalid"))?;
    projected_request.remove("bytes");
    projected_request.insert(
        "bytes_path".to_string(),
        Value::String(format!("{STORE_CHUNK_OBJECT_DIRECTORY}/{bytes_sha256}")),
    );
    projected_request.insert("bytes_sha256".to_string(), Value::String(bytes_sha256));
    projected_request.insert("bytes_size".to_string(), Value::from(bytes_size));
    Ok(projection)
}

fn insert_serialized<T>(
    projection: &mut Map<String, Value>,
    field: &'static str,
    value: &T,
) -> Result<(), serde_json::Error>
where
    T: Serialize,
{
    projection.insert(field.to_string(), to_value(value)?);
    Ok(())
}

fn json_shape_error(reason: &'static str) -> serde_json::Error {
    <serde_json::Error as serde::ser::Error>::custom(reason)
}

pub(super) fn to_vec(value: &impl Serialize) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = Vec::new();
    let mut serializer = serde_json::Serializer::new(&mut bytes);
    value.serialize(DecimalU128Serializer(&mut serializer))?;
    Ok(bytes)
}

pub(super) fn to_value(value: &impl Serialize) -> Result<serde_json::Value, serde_json::Error> {
    value.serialize(DecimalU128Serializer(serde_json::value::Serializer))
}

struct DecimalU128Value<'a, T: ?Sized>(&'a T);

impl<T> Serialize for DecimalU128Value<'_, T>
where
    T: Serialize + ?Sized,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(DecimalU128Serializer(serializer))
    }
}

struct DecimalU128Serializer<S>(S);

impl<S> Serializer for DecimalU128Serializer<S>
where
    S: Serializer,
{
    type Ok = S::Ok;
    type Error = S::Error;
    type SerializeSeq = DecimalU128Seq<S::SerializeSeq>;
    type SerializeTuple = DecimalU128Tuple<S::SerializeTuple>;
    type SerializeTupleStruct = DecimalU128TupleStruct<S::SerializeTupleStruct>;
    type SerializeTupleVariant = DecimalU128TupleVariant<S::SerializeTupleVariant>;
    type SerializeMap = DecimalU128Map<S::SerializeMap>;
    type SerializeStruct = DecimalU128Struct<S::SerializeStruct>;
    type SerializeStructVariant = DecimalU128StructVariant<S::SerializeStructVariant>;

    fn serialize_bool(self, value: bool) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_bool(value)
    }

    fn serialize_i8(self, value: i8) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_i8(value)
    }

    fn serialize_i16(self, value: i16) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_i16(value)
    }

    fn serialize_i32(self, value: i32) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_i32(value)
    }

    fn serialize_i64(self, value: i64) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_i64(value)
    }

    fn serialize_i128(self, value: i128) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_i128(value)
    }

    fn serialize_u8(self, value: u8) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_u8(value)
    }

    fn serialize_u16(self, value: u16) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_u16(value)
    }

    fn serialize_u32(self, value: u32) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_u32(value)
    }

    fn serialize_u64(self, value: u64) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_u64(value)
    }

    fn serialize_u128(self, value: u128) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_str(&value.to_string())
    }

    fn serialize_f32(self, value: f32) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_f32(value)
    }

    fn serialize_f64(self, value: f64) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_f64(value)
    }

    fn serialize_char(self, value: char) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_char(value)
    }

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_str(value)
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_bytes(value)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_none()
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.0.serialize_some(&DecimalU128Value(value))
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_unit()
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_unit_struct(name)
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_unit_variant(name, variant_index, variant)
    }

    fn serialize_newtype_struct<T>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.0
            .serialize_newtype_struct(name, &DecimalU128Value(value))
    }

    fn serialize_newtype_variant<T>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.0
            .serialize_newtype_variant(name, variant_index, variant, &DecimalU128Value(value))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.0.serialize_seq(len).map(DecimalU128Seq)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.0.serialize_tuple(len).map(DecimalU128Tuple)
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.0
            .serialize_tuple_struct(name, len)
            .map(DecimalU128TupleStruct)
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.0
            .serialize_tuple_variant(name, variant_index, variant, len)
            .map(DecimalU128TupleVariant)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.0.serialize_map(len).map(DecimalU128Map)
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.0.serialize_struct(name, len).map(DecimalU128Struct)
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.0
            .serialize_struct_variant(name, variant_index, variant, len)
            .map(DecimalU128StructVariant)
    }

    fn collect_str<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Display + ?Sized,
    {
        self.0.collect_str(value)
    }

    fn is_human_readable(&self) -> bool {
        self.0.is_human_readable()
    }
}

struct DecimalU128Seq<S>(S);

impl<S> SerializeSeq for DecimalU128Seq<S>
where
    S: SerializeSeq,
{
    type Ok = S::Ok;
    type Error = S::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.0.serialize_element(&DecimalU128Value(value))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.0.end()
    }
}

struct DecimalU128Tuple<S>(S);

impl<S> SerializeTuple for DecimalU128Tuple<S>
where
    S: SerializeTuple,
{
    type Ok = S::Ok;
    type Error = S::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.0.serialize_element(&DecimalU128Value(value))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.0.end()
    }
}

struct DecimalU128TupleStruct<S>(S);

impl<S> SerializeTupleStruct for DecimalU128TupleStruct<S>
where
    S: SerializeTupleStruct,
{
    type Ok = S::Ok;
    type Error = S::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.0.serialize_field(&DecimalU128Value(value))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.0.end()
    }
}

struct DecimalU128TupleVariant<S>(S);

impl<S> SerializeTupleVariant for DecimalU128TupleVariant<S>
where
    S: SerializeTupleVariant,
{
    type Ok = S::Ok;
    type Error = S::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.0.serialize_field(&DecimalU128Value(value))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.0.end()
    }
}

struct DecimalU128Map<S>(S);

impl<S> SerializeMap for DecimalU128Map<S>
where
    S: SerializeMap,
{
    type Ok = S::Ok;
    type Error = S::Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.0.serialize_key(&DecimalU128Value(key))
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.0.serialize_value(&DecimalU128Value(value))
    }

    fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<(), Self::Error>
    where
        K: Serialize + ?Sized,
        V: Serialize + ?Sized,
    {
        self.0
            .serialize_entry(&DecimalU128Value(key), &DecimalU128Value(value))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.0.end()
    }
}

struct DecimalU128Struct<S>(S);

impl<S> SerializeStruct for DecimalU128Struct<S>
where
    S: SerializeStruct,
{
    type Ok = S::Ok;
    type Error = S::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.0.serialize_field(key, &DecimalU128Value(value))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.0.end()
    }
}

struct DecimalU128StructVariant<S>(S);

impl<S> SerializeStructVariant for DecimalU128StructVariant<S>
where
    S: SerializeStructVariant,
{
    type Ok = S::Ok;
    type Error = S::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.0.serialize_field(key, &DecimalU128Value(value))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.0.end()
    }
}
