//! A JSON Schema checker for the keyword subset the published schemas use:
//! `type` (a name or an array of names), `properties`, `required`,
//! `additionalProperties` (boolean), `items`, `enum`, and `$ref` to a
//! sibling file. Any other keyword is an error, so a schema cannot rely on a
//! constraint this checker does not enforce.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

const TYPES: [&str; 7] = [
    "object", "array", "string", "integer", "number", "boolean", "null",
];

/// The published JSON v1 schemas.
pub fn schema_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/v1")
}

/// Checks `instance` against the published schema file `file`.
pub fn check_published(file: &str, instance: &Value) -> Result<(), String> {
    check(&schema_directory(), file, instance)
}

/// Checks `instance` against `directory/file`. The schema, and every schema
/// it refers to, is first checked to use only the supported keywords, whether
/// or not `instance` reaches them.
pub fn check(directory: &Path, file: &str, instance: &Value) -> Result<(), String> {
    validate_schema_file(directory, file, &mut BTreeSet::new())?;
    check_node(directory, &load(directory, file)?, instance, "$")
}

fn load(directory: &Path, file: &str) -> Result<Value, String> {
    let bytes = fs::read(directory.join(file))
        .map_err(|error| format!("{file}: cannot read the schema: {error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("{file}: not JSON: {error}"))
}

fn validate_schema_file(
    directory: &Path,
    file: &str,
    visited: &mut BTreeSet<String>,
) -> Result<(), String> {
    if !visited.insert(file.to_owned()) {
        return Ok(());
    }
    validate_schema(directory, &load(directory, file)?, file, visited)
}

fn validate_schema(
    directory: &Path,
    schema: &Value,
    location: &str,
    visited: &mut BTreeSet<String>,
) -> Result<(), String> {
    let Some(keywords) = schema.as_object() else {
        return Err(format!("{location}: a schema must be an object"));
    };
    if let Some(reference) = keywords.get("$ref") {
        if keywords.len() != 1 {
            return Err(format!("{location}: $ref must be the only keyword"));
        }
        return validate_schema_file(directory, sibling(reference, location)?, visited);
    }
    for (keyword, value) in keywords {
        let location = format!("{location}/{keyword}");
        match keyword.as_str() {
            "type" => {
                let names = match value {
                    Value::String(_) => std::slice::from_ref(value),
                    Value::Array(names) if !names.is_empty() => names.as_slice(),
                    _ => return Err(format!("{location}: expected a type name or names")),
                };
                for name in names {
                    if !name.as_str().is_some_and(|name| TYPES.contains(&name)) {
                        return Err(format!("{location}: unknown type {name}"));
                    }
                }
            }
            "properties" => {
                let Some(properties) = value.as_object() else {
                    return Err(format!("{location}: expected an object"));
                };
                for (name, property) in properties {
                    validate_schema(directory, property, &format!("{location}/{name}"), visited)?;
                }
            }
            "required" => {
                if !value
                    .as_array()
                    .is_some_and(|names| names.iter().all(Value::is_string))
                {
                    return Err(format!("{location}: expected an array of names"));
                }
            }
            "additionalProperties" => {
                if !value.is_boolean() {
                    return Err(format!("{location}: expected a boolean"));
                }
            }
            "items" => validate_schema(directory, value, &location, visited)?,
            "enum" => {
                if !value.as_array().is_some_and(|values| !values.is_empty()) {
                    return Err(format!("{location}: expected a non-empty array"));
                }
            }
            _ => return Err(format!("{location}: unsupported keyword")),
        }
    }
    Ok(())
}

/// The file a `$ref` names. Only a plain file name in the same directory is
/// allowed: no path, no URL and no fragment.
fn sibling<'a>(reference: &'a Value, location: &str) -> Result<&'a str, String> {
    reference
        .as_str()
        .filter(|file| {
            !file.is_empty() && !file.starts_with('.') && !file.contains(['/', '\\', '#', ':'])
        })
        .ok_or_else(|| format!("{location}: $ref must name a sibling file"))
}

fn check_node(
    directory: &Path,
    schema: &Value,
    instance: &Value,
    location: &str,
) -> Result<(), String> {
    if let Some(reference) = schema.get("$ref") {
        let file = sibling(reference, location)?;
        return check_node(directory, &load(directory, file)?, instance, location);
    }
    if let Some(types) = schema.get("type") {
        let allowed = match types {
            Value::Array(names) => names.iter().any(|name| has_type(instance, name)),
            name => has_type(instance, name),
        };
        if !allowed {
            return Err(format!(
                "{location}: expected type {types}, found {instance}"
            ));
        }
    }
    if let Some(values) = schema.get("enum").and_then(Value::as_array)
        && !values.contains(instance)
    {
        return Err(format!(
            "{location}: {instance} is not one of the allowed values"
        ));
    }
    if let Some(object) = instance.as_object() {
        let properties = schema.get("properties").and_then(Value::as_object);
        for name in schema
            .get("required")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !object.contains_key(name) {
                return Err(format!("{location}: missing required key {name:?}"));
            }
        }
        let closed = schema.get("additionalProperties") == Some(&Value::Bool(false));
        for (name, value) in object {
            match properties.and_then(|properties| properties.get(name)) {
                Some(property) => {
                    check_node(directory, property, value, &format!("{location}.{name}"))?;
                }
                None if closed => return Err(format!("{location}: unexpected key {name:?}")),
                None => {}
            }
        }
    }
    if let (Some(items), Some(elements)) = (schema.get("items"), instance.as_array()) {
        for (index, element) in elements.iter().enumerate() {
            check_node(directory, items, element, &format!("{location}[{index}]"))?;
        }
    }
    Ok(())
}

fn has_type(instance: &Value, name: &Value) -> bool {
    match name.as_str() {
        Some("object") => instance.is_object(),
        Some("array") => instance.is_array(),
        Some("string") => instance.is_string(),
        Some("integer") => instance.is_i64() || instance.is_u64(),
        Some("number") => instance.is_number(),
        Some("boolean") => instance.is_boolean(),
        Some("null") => instance.is_null(),
        _ => false,
    }
}

/// Checks that `directory/file`, and every schema it refers to, uses only
/// the supported keywords.
pub fn validate(directory: &Path, file: &str) -> Result<(), String> {
    validate_schema_file(directory, file, &mut BTreeSet::new())
}
