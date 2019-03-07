use rustc_hex::ToHex;
use serde::ser::SerializeSeq;
use serde::ser::SerializeStruct;
use serde::Serialize;
use serde::Serializer;

use crate::types::EntryOwned;
use crate::types::NodeDiffOwned;

struct PrefixedFormat<T: SerializePrefixedFormat>(T);

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

impl SerializePrefixedFormat for EntryOwned {
    fn serialize_prefixed_format<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("EntryOwned", 2)?;
        s.serialize_field("key", &format!("0x{}", self.key.inner.to_hex::<String>()))?;
        s.serialize_field("value", &format!("0x{}", self.value.to_hex::<String>()))?;
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

#[cfg(test)]
mod tests {
    use super::PrefixedFormat;
    use rustc_hex::FromHex;
    use serde_json::Value as JsonValue;

    #[test]
    fn json_serialize_prefixed_format_entry_owned() {
        let entry = PrefixedFormat(crate::types::EntryOwned {
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
}
