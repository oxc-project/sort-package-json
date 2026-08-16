use std::{borrow::Cow, fmt, marker::PhantomData};

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{Error, MapAccess, SeqAccess, Visitor},
    ser::{SerializeMap, SerializeSeq},
};
use serde_json::Number;

pub(crate) type Object<'a> = Vec<(Cow<'a, str>, Value<'a>)>;

/// A JSON value that borrows strings without escape sequences from the input.
///
/// `serde_json::Value` owns every string and stores objects in an `IndexMap` when
/// `preserve_order` is enabled. Package manifests are short-lived and are sorted
/// immediately, so a borrowing `Cow` plus `Vec` representation avoids that work.
pub(crate) enum Value<'a> {
    Null,
    Bool(bool),
    Number(Number),
    String(Cow<'a, str>),
    Array(Vec<Value<'a>>),
    Object(Object<'a>),
}

impl Value<'_> {
    #[inline]
    pub(crate) fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    #[inline]
    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }
}

impl Serialize for Value<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Bool(value) => serializer.serialize_bool(*value),
            Self::Number(value) => value.serialize(serializer),
            Self::String(value) => serializer.serialize_str(value),
            Self::Array(values) => {
                let mut seq = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    seq.serialize_element(value)?;
                }
                seq.end()
            }
            Self::Object(entries) => {
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
        }
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for Value<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor(PhantomData))
    }
}

struct ValueVisitor<'a>(PhantomData<&'a ()>);

impl<'de: 'a, 'a> Visitor<'de> for ValueVisitor<'a> {
    type Value = Value<'a>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any valid JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Number::from_f64(value).map(Value::Number).ok_or_else(|| E::custom("not a JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Value::String(Cow::Owned(value.to_owned())))
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E> {
        Ok(Value::String(Cow::Borrowed(value)))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(Value::String(Cow::Owned(value)))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(value) = seq.next_element()? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut entries: Object<'a> = Vec::with_capacity(map.size_hint().unwrap_or(0));
        let mut key_fingerprints = 0_u128;

        while let Some(Key(key)) = map.next_key::<Key<'a>>()? {
            let value = map.next_value()?;

            // JSON permits duplicate keys. Match `serde_json::Map` by retaining the
            // first position and replacing its value. The fingerprint is only a
            // fast rejection filter; collisions fall back to an exact comparison.
            let fingerprint =
                key.bytes().fold(0_u8, |hash, byte| hash.wrapping_mul(31).wrapping_add(byte)) & 127;
            let mask = 1_u128 << fingerprint;
            if key_fingerprints & mask != 0 {
                if let Some((_, previous)) =
                    entries.iter_mut().find(|(previous, _)| previous == &key)
                {
                    *previous = value;
                    continue;
                }
            }
            key_fingerprints |= mask;
            entries.push((key, value));
        }

        Ok(Value::Object(entries))
    }
}

struct Key<'a>(Cow<'a, str>);

impl<'de: 'a, 'a> Deserialize<'de> for Key<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(KeyVisitor(PhantomData))
    }
}

struct KeyVisitor<'a>(PhantomData<&'a ()>);

impl<'de: 'a, 'a> Visitor<'de> for KeyVisitor<'a> {
    type Value = Key<'a>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a string key")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Key(Cow::Owned(value.to_owned())))
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E> {
        Ok(Key(Cow::Borrowed(value)))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(Key(Cow::Owned(value)))
    }
}
