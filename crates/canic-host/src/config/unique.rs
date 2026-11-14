//! Reject duplicate JSON names instead of silently replacing app/fleet/slot declarations.
use serde::{
    Deserialize, Deserializer,
    de::{self, MapAccess, Visitor},
};
use std::{collections::BTreeMap, fmt, marker::PhantomData};

pub(super) fn map<'de, D, T>(deserializer: D) -> Result<BTreeMap<String, T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Unique<T>(PhantomData<T>);

    impl<'de, T: Deserialize<'de>> Visitor<'de> for Unique<T> {
        type Value = BTreeMap<String, T>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an object with unique configuration names")
        }

        fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
            let mut values = BTreeMap::new();
            while let Some(key) = access.next_key::<String>()? {
                if values.contains_key(&key) {
                    return Err(de::Error::custom("duplicate configuration name"));
                }
                values.insert(key, access.next_value()?);
            }
            Ok(values)
        }
    }

    deserializer.deserialize_map(Unique(PhantomData))
}
