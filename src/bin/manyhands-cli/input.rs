//! Read-once mutation input. Values never implement diagnostic formatting.

use std::{collections::BTreeMap, fmt, fs, io::Read, path::Path};

use serde::{
    Deserialize, Deserializer,
    de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor},
};

use super::render;

pub struct InputObject {
    fields: BTreeMap<String, Json>,
}

pub enum Tristate {
    Missing,
    Null,
    Value(String),
}

pub enum InputError {
    Unreadable,
    NotUtf8,
    Empty,
    Malformed,
    NotObject,
    DuplicateKey(String),
    UnknownField(String),
    Missing(&'static str),
    WrongType {
        field: &'static str,
        expected: &'static str,
    },
    InvalidValue {
        field: &'static str,
        allowed: &'static [&'static str],
    },
}

impl InputError {
    pub fn detail(&self) -> String {
        match self {
            Self::Unreadable => "The input could not be read.".into(),
            Self::NotUtf8 => "The input is not UTF-8.".into(),
            Self::Empty => "The input is empty.".into(),
            Self::Malformed => "The input must be exactly one valid JSON object.".into(),
            Self::NotObject => "The input must be a JSON object.".into(),
            Self::DuplicateKey(key) => format!("Duplicate input field: {}.", render::field(key)),
            Self::UnknownField(key) => format!("Unknown input field: {}.", render::field(key)),
            Self::Missing(field) => {
                format!("Required input field is missing: {}.", render::field(field))
            }
            Self::WrongType { field, expected } => format!(
                "Input field {} must be {}.",
                render::field(field),
                render::field(expected)
            ),
            Self::InvalidValue { field, allowed } => format!(
                "Input field {} must be one of: {}.",
                render::field(field),
                allowed
                    .iter()
                    .map(|value| render::field(value))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

pub fn read(source: &str, cwd: &Path, stdin: &mut dyn Read) -> Result<InputObject, InputError> {
    let bytes = if source == "-" {
        let mut bytes = Vec::new();
        stdin
            .read_to_end(&mut bytes)
            .map_err(|_| InputError::Unreadable)?;
        bytes
    } else {
        fs::read(cwd.join(source)).map_err(|_| InputError::Unreadable)?
    };
    parse(&bytes)
}

pub fn parse(bytes: &[u8]) -> Result<InputObject, InputError> {
    let text = std::str::from_utf8(bytes).map_err(|_| InputError::NotUtf8)?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    if text
        .bytes()
        .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
    {
        return Err(InputError::Empty);
    }
    // Keep serde_json's default recursion limit. The seed carries only a duplicate
    // key out of the visitor: no parsing or forwarding of serde's error text.
    let mut duplicate = None;
    let mut deserializer = serde_json::Deserializer::from_str(text);
    let value = JsonSeed {
        duplicate: &mut duplicate,
    }
    .deserialize(&mut deserializer)
    .map_err(|_| duplicate.map_or(InputError::Malformed, InputError::DuplicateKey))?;
    deserializer.end().map_err(|_| InputError::Malformed)?;
    match value {
        Json::Object(fields) => Ok(InputObject { fields }),
        _ => Err(InputError::NotObject),
    }
}

impl InputObject {
    pub fn only(&self, allowed: &[&str]) -> Result<(), InputError> {
        for key in self.fields.keys() {
            if !allowed.contains(&key.as_str()) {
                return Err(InputError::UnknownField(key.clone()));
            }
        }
        Ok(())
    }

    pub fn string(&self, field: &'static str) -> Result<String, InputError> {
        self.optional_string(field)?
            .ok_or(InputError::Missing(field))
    }

    pub fn optional_string(&self, field: &'static str) -> Result<Option<String>, InputError> {
        match self.fields.get(field) {
            None => Ok(None),
            Some(Json::String(value)) => Ok(Some(value.clone())),
            _ => Err(InputError::WrongType {
                field,
                expected: "a string",
            }),
        }
    }

    pub fn nullable_string(&self, field: &'static str) -> Result<Option<String>, InputError> {
        match self.tristate(field)? {
            Tristate::Missing => Err(InputError::Missing(field)),
            Tristate::Null => Ok(None),
            Tristate::Value(value) => Ok(Some(value)),
        }
    }

    pub fn tristate(&self, field: &'static str) -> Result<Tristate, InputError> {
        match self.fields.get(field) {
            None => Ok(Tristate::Missing),
            Some(Json::Null) => Ok(Tristate::Null),
            Some(Json::String(value)) => Ok(Tristate::Value(value.clone())),
            _ => Err(InputError::WrongType {
                field,
                expected: "a string or null",
            }),
        }
    }

    pub fn optional_object(&self, field: &'static str) -> Result<Option<InputObject>, InputError> {
        match self.fields.get(field) {
            None => Ok(None),
            Some(Json::Object(fields)) => Ok(Some(InputObject {
                fields: fields.clone(),
            })),
            _ => Err(InputError::WrongType {
                field,
                expected: "an object",
            }),
        }
    }

    pub fn one_of(
        &self,
        field: &'static str,
        allowed: &'static [&'static str],
    ) -> Result<&'static str, InputError> {
        let value = self.string(field)?;
        allowed
            .iter()
            .copied()
            .find(|allowed| *allowed == value)
            .ok_or(InputError::InvalidValue { field, allowed })
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn schema_name(resource: &str, verb: &str) -> String {
    format!("{resource}_{}.schema.json", verb.replace('-', "_"))
}

/// Retain the entire JSON tree, including types current accessors only refuse.
#[derive(Clone)]
enum Json {
    Null,
    Bool(#[allow(dead_code)] bool),
    Number(#[allow(dead_code)] serde_json::Number),
    String(String),
    Array(#[allow(dead_code)] Vec<Json>),
    Object(BTreeMap<String, Json>),
}

impl<'de> Deserialize<'de> for Json {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        JsonSeed {
            duplicate: &mut None,
        }
        .deserialize(deserializer)
    }
}

struct JsonSeed<'a> {
    duplicate: &'a mut Option<String>,
}

impl<'de> DeserializeSeed<'de> for JsonSeed<'_> {
    type Value = Json;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Json, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for JsonSeed<'_> {
    type Value = Json;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_unit<E: de::Error>(self) -> Result<Json, E> {
        Ok(Json::Null)
    }
    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Json, E> {
        Ok(Json::Bool(value))
    }
    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Json, E> {
        Ok(Json::Number(value.into()))
    }
    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Json, E> {
        Ok(Json::Number(value.into()))
    }
    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Json, E> {
        serde_json::Number::from_f64(value)
            .map(Json::Number)
            .ok_or_else(|| E::custom("invalid JSON number"))
    }
    fn visit_str<E: de::Error>(self, value: &str) -> Result<Json, E> {
        Ok(Json::String(value.to_owned()))
    }
    fn visit_string<E: de::Error>(self, value: String) -> Result<Json, E> {
        Ok(Json::String(value))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Json, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(JsonSeed {
            duplicate: self.duplicate,
        })? {
            values.push(value);
        }
        Ok(Json::Array(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Json, A::Error> {
        let mut fields = BTreeMap::new();
        while let Some(key) = map.next_key::<String>()? {
            if fields.contains_key(&key) {
                *self.duplicate = Some(key);
                return Err(de::Error::custom("duplicate JSON key"));
            }
            let value = map.next_value_seed(JsonSeed {
                duplicate: self.duplicate,
            })?;
            fields.insert(key, value);
        }
        Ok(Json::Object(fields))
    }
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;
