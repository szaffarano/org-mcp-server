mod org_agenda;
mod org_capture;
mod org_file_list;
mod org_search;
mod org_update_todo;

use rmcp::schemars::Schema;
use serde_json::{Map, Value, json};

const ANNOTATION_KEYS: [&str; 7] = [
    "title",
    "description",
    "default",
    "deprecated",
    "examples",
    "readOnly",
    "writeOnly",
];

/// Rewrites `"type": ["<T>", "null"]` properties as `anyOf: [{"type": "<T>", ...}, {"type": "null"}]`,
/// since several clients only understand a single `type` per schema.
pub(crate) fn nullable_as_any_of(schema: &mut Schema) {
    let Some(properties) = schema.get_mut("properties").and_then(Value::as_object_mut) else {
        return;
    };

    for property in properties.values_mut().filter_map(Value::as_object_mut) {
        split_nullable(property);
    }
}

fn split_nullable(property: &mut Map<String, Value>) {
    let Some(Value::Array(types)) = property.get("type") else {
        return;
    };
    let [Value::String(a), Value::String(b)] = types.as_slice() else {
        return;
    };
    let typed = match (a.as_str(), b.as_str()) {
        ("null", other) | (other, "null") if other != "null" => other.to_string(),
        _ => return,
    };

    let mut branch = Map::new();
    branch.insert("type".to_string(), Value::String(typed));
    property.remove("type");

    let moved: Vec<String> = property
        .keys()
        .filter(|key| !ANNOTATION_KEYS.contains(&key.as_str()))
        .cloned()
        .collect();
    for key in moved {
        if let Some(value) = property.remove(&key) {
            branch.insert(key, value);
        }
    }

    property.insert(
        "anyOf".to_string(),
        json!([Value::Object(branch), {"type": "null"}]),
    );
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use crate::core::OrgModeRouter;

    fn find_type_arrays(value: &Value, path: &str, found: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                if matches!(map.get("type"), Some(Value::Array(_))) {
                    found.push(path.to_string());
                }
                for (key, child) in map {
                    find_type_arrays(child, &format!("{path}.{key}"), found);
                }
            }
            Value::Array(items) => {
                for (i, child) in items.iter().enumerate() {
                    find_type_arrays(child, &format!("{path}[{i}]"), found);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn test_tool_input_schemas_have_no_type_arrays() {
        let mut found = Vec::new();
        for tool in OrgModeRouter::tool_router().list_all() {
            let schema = Value::Object((*tool.input_schema).clone());
            find_type_arrays(&schema, &tool.name, &mut found);
        }
        assert_eq!(found, Vec::<String>::new());
    }

    #[test]
    fn test_nullable_property_keeps_constraints_in_typed_branch() {
        let tool = OrgModeRouter::tool_router().get("org-search").cloned();
        let schema = Value::Object((*tool.unwrap().input_schema).clone());

        assert_eq!(
            schema["properties"]["limit"]["anyOf"],
            serde_json::json!([
                {"type": "integer", "format": "uint", "minimum": 0},
                {"type": "null"}
            ])
        );
        assert_eq!(
            schema["properties"]["limit"]["description"],
            "Maximum number of search results to return (optional)"
        );
        assert_eq!(
            schema["properties"]["tags"]["anyOf"],
            serde_json::json!([
                {"type": "array", "items": {"type": "string"}},
                {"type": "null"}
            ])
        );
    }
}
