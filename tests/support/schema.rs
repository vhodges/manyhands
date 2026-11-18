//! A JSON Schema checker for the keyword subset the published schemas use:
//! `type` (a name or an array of names), `properties`, `required`,
//! `additionalProperties` (boolean), `items`, `enum`, `minimum`, and `$ref`
//! to a sibling file. The annotations `$schema`, `$id`, `title` and `description`
//! are accepted and ignored. Any other keyword is an error, so a schema
//! cannot rely on a constraint this checker does not enforce.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

const TYPES: [&str; 7] = [
    "object", "array", "string", "integer", "number", "boolean", "null",
];

/// Keywords that say what a schema is and constrain nothing.
const ANNOTATIONS: [&str; 4] = ["$schema", "$id", "title", "description"];

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
    validate(directory, file)?;
    check_node(directory, &load(directory, file)?, instance, "$")
}

/// Checks that `directory/file`, and every schema it refers to, uses only
/// the supported keywords, each in a form this checker enforces.
pub fn validate(directory: &Path, file: &str) -> Result<(), String> {
    validate_schema_file(directory, file, &mut BTreeSet::new())
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
    for annotation in ANNOTATIONS {
        if keywords
            .get(annotation)
            .is_some_and(|text| !text.is_string())
        {
            return Err(format!("{location}/{annotation}: expected a string"));
        }
    }
    if let Some(reference) = keywords.get("$ref") {
        if keywords
            .keys()
            .any(|keyword| keyword != "$ref" && !ANNOTATIONS.contains(&keyword.as_str()))
        {
            return Err(format!("{location}: $ref must be the only keyword"));
        }
        return validate_schema_file(directory, sibling(reference, location)?, visited);
    }

    // A keyword that applies to one type is refused where the schema's own
    // `type` rules that type out: it would constrain nothing.
    let allows = |name: &str| match keywords.get("type") {
        Some(Value::Array(names)) => names.iter().any(|allowed| allowed == name),
        Some(allowed) => allowed == name,
        None => true,
    };
    let properties = keywords.get("properties").and_then(Value::as_object);
    for (keyword, value) in keywords {
        let location = format!("{location}/{keyword}");
        match keyword.as_str() {
            keyword if ANNOTATIONS.contains(&keyword) => {}
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
            "properties" | "required" | "additionalProperties" if !allows("object") => {
                return Err(format!("{location}: the type cannot be an object"));
            }
            "items" if !allows("array") => {
                return Err(format!("{location}: the type cannot be an array"));
            }
            "minimum" if !allows("integer") && !allows("number") => {
                return Err(format!("{location}: the type cannot be a number"));
            }
            "minimum" => {
                if !value.is_number() {
                    return Err(format!("{location}: expected a number"));
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
                let Some(names) = value.as_array() else {
                    return Err(format!("{location}: expected an array of names"));
                };
                for name in names {
                    let Some(name) = name.as_str() else {
                        return Err(format!("{location}: expected an array of names"));
                    };
                    if !properties.is_some_and(|properties| properties.contains_key(name)) {
                        return Err(format!("{location}: {name:?} is not a property"));
                    }
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
        && !values.iter().any(|value| same_value(value, instance))
    {
        return Err(format!(
            "{location}: {instance} is not one of the allowed values"
        ));
    }
    if let (Some(minimum), Some(number)) = (schema.get("minimum"), instance.as_number())
        && is_less(number, minimum)
    {
        return Err(format!(
            "{location}: {instance} is less than the minimum {minimum}"
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

/// Equality as JSON Schema has it: a number equals a number of the same
/// value however it is written, so `1` and `1.0` are one value.
fn same_value(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => {
            left == right || left.as_f64() == right.as_f64()
        }
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| same_value(left, right))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .all(|(key, left)| right.get(key).is_some_and(|right| same_value(left, right)))
        }
        _ => left == right,
    }
}

/// Whether `number` is below `minimum`. Two integers are compared as
/// integers, so that none is rounded on the way.
fn is_less(number: &serde_json::Number, minimum: &Value) -> bool {
    match (
        number.as_i128(),
        minimum.as_number().and_then(serde_json::Number::as_i128),
    ) {
        (Some(number), Some(minimum)) => number < minimum,
        _ => number.as_f64() < minimum.as_f64(),
    }
}

fn has_type(instance: &Value, name: &Value) -> bool {
    match name.as_str() {
        Some("object") => instance.is_object(),
        Some("array") => instance.is_array(),
        Some("string") => instance.is_string(),
        Some("integer") => {
            instance.is_i64()
                || instance.is_u64()
                || instance
                    .as_f64()
                    .is_some_and(|number| number.fract() == 0.0)
        }
        Some("number") => instance.is_number(),
        Some("boolean") => instance.is_boolean(),
        Some("null") => instance.is_null(),
        _ => false,
    }
}
