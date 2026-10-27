//! Native metadata for the metric selector's unchanged Serde representation.

use super::PublicMetricFamily;
use ic_wire::typescript::{GenerationError, SchemaContext, Wire};
use serde_json::{Value, json};

impl Wire for PublicMetricFamily {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        // Serde names remain configuration contracts; native tags follow declaration order.
        Ok(json!({"k":"enum", "variants":[
            {"name":"application", "id":0, "value":{"k":"unit"}},
            {"name":"cycles", "id":1, "value":{"k":"unit"}},
            {"name":"operations", "id":2, "value":{"k":"unit"}},
            {"name":"performance", "id":3, "value":{"k":"unit"}},
            {"name":"shard_occupancy", "id":4, "value":{"k":"unit"}}
        ]}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::public_metrics::{PublicMetricKind, TimerMetricRegistration};
    use ic_wire::{Limits, native};

    #[test]
    fn family_projection_preserves_configuration_names_and_native_tags() {
        let limits = Limits {
            bytes: 64,
            allocation: 1024,
            nodes: 16,
            depth: 4,
        };
        let schema = ic_wire::typescript::native_schema::<PublicMetricFamily>().unwrap();
        let variants = schema["variants"].as_array().unwrap();
        let families = [
            PublicMetricFamily::Application,
            PublicMetricFamily::Cycles,
            PublicMetricFamily::Operations,
            PublicMetricFamily::Performance,
            PublicMetricFamily::ShardOccupancy,
        ];
        assert_eq!(variants.len(), families.len());
        for (family, variant) in families.into_iter().zip(variants) {
            let name = serde_json::to_value(family).unwrap();
            assert_eq!(variant["name"], name);
            assert_eq!(
                serde_json::from_value::<PublicMetricFamily>(name).unwrap(),
                family
            );
            let bytes = native::encode(&family, limits).unwrap();
            assert_eq!(
                bytes,
                native::encode(&variant["id"].as_u64().unwrap(), limits).unwrap()
            );
            assert_eq!(
                native::decode::<PublicMetricFamily>(&bytes, limits).unwrap(),
                family
            );
            assert_eq!(variant["value"], json!({"k":"unit"}));
        }
        let invalid = native::encode(&5_u8, limits).unwrap();
        assert!(native::decode::<PublicMetricFamily>(&invalid, limits).is_err());
    }

    #[test]
    fn metric_kind_projection_preserves_counter_lifetimes() {
        let limits = Limits {
            bytes: 256,
            allocation: 4096,
            nodes: 64,
            depth: 8,
        };
        let schema = ic_wire::typescript::native_schema::<PublicMetricKind>().unwrap();
        let variants = schema["variants"].as_array().unwrap();
        let values = [
            PublicMetricKind::Gauge,
            PublicMetricKind::Counter {
                window_id: u64::MAX,
                saturated: true,
            },
            PublicMetricKind::TimerCounter {
                registration: TimerMetricRegistration {
                    canister_version: 7,
                    started_at_ns: u64::MAX,
                    sequence: 42,
                },
                saturated: false,
            },
        ];
        for (value, variant) in values.into_iter().zip(variants) {
            let bytes = native::encode(&value, limits).unwrap();
            assert_eq!(u64::from(bytes[0]), variant["id"].as_u64().unwrap());
            assert_eq!(
                native::decode::<PublicMetricKind>(&bytes, limits).unwrap(),
                value
            );
        }
        assert_eq!(
            variants[2]["value"]["fields"][0]["value"],
            ic_wire::typescript::native_schema::<TimerMetricRegistration>().unwrap()
        );
    }
}
