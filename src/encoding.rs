use rustc_hex::{FromHex, ToHex};
use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};
use serde::ser::{Serialize, SerializeSeq, SerializeStruct, Serializer};
use trie_db::NibbleSlice;

use crate::types::EntryOwned;
use crate::types::NodeDiffOwned;

struct PrefixedFormat<T>(T);

pub trait SerializePrefixedFormat {
    fn serialize_prefixed_format<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
}

impl<T: SerializePrefixedFormat> Serialize for PrefixedFormat<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        SerializePrefixedFormat::serialize_prefixed_format(&self.0, serializer)
    }
}

impl<T: SerializePrefixedFormat> SerializePrefixedFormat for &T {
    fn serialize_prefixed_format<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        SerializePrefixedFormat::serialize_prefixed_format(*self, serializer)
    }
}

impl<T: SerializePrefixedFormat> SerializePrefixedFormat for Vec<T> {
    fn serialize_prefixed_format<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for entry in self.iter() {
            seq.serialize_element(&PrefixedFormat(entry))?;
        }
        seq.end()
    }
}

impl SerializePrefixedFormat for Vec<u8> {
    fn serialize_prefixed_format<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("0x{}", self.to_hex::<String>()))
    }
}

impl SerializePrefixedFormat for EntryOwned {
    fn serialize_prefixed_format<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("EntryOwned", 2)?;
        s.serialize_field("key", &PrefixedFormat(&self.key.inner))?;
        s.serialize_field("value", &PrefixedFormat(&self.value))?;
        s.end()
    }
}

impl SerializePrefixedFormat for NodeDiffOwned {
    fn serialize_prefixed_format<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("NodeDiffOwned", 2)?;
        s.serialize_field("added_entries", &PrefixedFormat(&self.added_entries))?;
        s.serialize_field("removed_entries", &PrefixedFormat(&self.removed_entries))?;

        s.end()
    }
}

pub trait DeserializePrefixedFormat<'de>: Sized {
    fn deserialize_prefixed_format<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>;
}

impl<'de, T: DeserializePrefixedFormat<'de>> Deserialize<'de> for PrefixedFormat<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        DeserializePrefixedFormat::deserialize_prefixed_format(deserializer)
    }
}

impl<'de, T: DeserializePrefixedFormat<'de>> DeserializePrefixedFormat<'de> for PrefixedFormat<T> {
    fn deserialize_prefixed_format<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(PrefixedFormat(
            DeserializePrefixedFormat::deserialize_prefixed_format(deserializer)?,
        ))
    }
}

impl<'de> DeserializePrefixedFormat<'de> for Vec<u8> {
    fn deserialize_prefixed_format<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringVisitor;

        impl<'de> Visitor<'de> for StringVisitor {
            type Value = Vec<u8>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a hex encoded string prefixed by 0x")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if &s[0..2] != "0x" {
                    return Err(de::Error::invalid_value(de::Unexpected::Str(s), &self));
                }
                Ok(s[2..].from_hex().map_err(de::Error::custom)?)
            }
        }

        deserializer.deserialize_str(StringVisitor)
    }
}

impl<'de> DeserializePrefixedFormat<'de> for EntryOwned {
    fn deserialize_prefixed_format<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StructVisitor;

        impl<'de> Visitor<'de> for StructVisitor {
            type Value = EntryOwned;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "struct EntryOwned")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut val_key = None;
                let mut val_value = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        "key" => {
                            if val_key.is_some() {
                                return Err(de::Error::duplicate_field("key"));
                            }
                            let inner_val: PrefixedFormat<Vec<u8>> = map.next_value()?;
                            let nibble = NibbleSlice::new(&inner_val.0);
                            val_key = Some(nibble.into());
                        }
                        "value" => {
                            if val_value.is_some() {
                                return Err(de::Error::duplicate_field("value"));
                            }
                            let inner_val: PrefixedFormat<Vec<u8>> = map.next_value()?;
                            val_value = Some(inner_val.0);
                        }
                        key => return Err(de::Error::unknown_field(key, FIELDS)),
                    }
                }
                let val_key = val_key.ok_or_else(|| de::Error::missing_field("key"))?;
                let val_value = val_value.ok_or_else(|| de::Error::missing_field("value"))?;
                Ok(EntryOwned {
                    key: val_key,
                    value: val_value,
                })
            }
        }

        const FIELDS: &'static [&'static str] = &["key", "value"];
        deserializer.deserialize_struct("Duration", FIELDS, StructVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::PrefixedFormat;
    use crate::types::EntryOwned;
    use rustc_hex::FromHex;
    use serde_json::Value as JsonValue;

    #[test]
    fn json_serialize_prefixed_format_entry_owned() {
        let entry = PrefixedFormat(EntryOwned {
            key: crate::types::NibbleOwned { inner: vec![] },
            value: b"baz".to_vec(),
        });
        let data = r#"
            {
                "key": "0x",
                "value": "0x62617a"
            }"#;
        let expected: JsonValue = serde_json::from_str(data).unwrap();
        let serialized = serde_json::to_value(entry).unwrap();

        assert_eq!(expected, serialized);
        let baz: Vec<u8> = serialized["value"].as_str().unwrap()[2..]
            .from_hex()
            .unwrap();
        assert_eq!("baz", std::str::from_utf8(&baz).unwrap())
    }

    #[test]
    fn json_deserialize_prefixed_format_vec_u8() {
        let expected = vec![0, 15, 16];
        let data = r#""0x000f10""#;
        let deserialized: PrefixedFormat<Vec<u8>> = serde_json::from_str(data).unwrap();

        assert_eq!(expected, deserialized.0);
    }

    #[test]
    fn json_deserialize_prefixed_format_entry_owned() {
        let expected = EntryOwned {
            key: crate::types::NibbleOwned { inner: vec![] },
            value: b"baz".to_vec(),
        };
        let data = r#"
            {
                "key": "0x",
                "value": "0x62617a"
            }"#;
        let deserialized: PrefixedFormat<EntryOwned> = serde_json::from_str(data).unwrap();

        assert_eq!(expected, deserialized.0);
    }
}
