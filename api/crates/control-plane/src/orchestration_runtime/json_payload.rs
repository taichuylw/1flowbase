use serde_json::{Map, Value};

pub(super) fn escape_json_nul_characters(value: Value) -> Value {
    match value {
        Value::String(text) => Value::String(escape_nul_characters(text)),
        Value::Array(items) => {
            Value::Array(items.into_iter().map(escape_json_nul_characters).collect())
        }
        Value::Object(entries) => Value::Object(
            entries
                .into_iter()
                .map(|(key, value)| {
                    (
                        escape_nul_characters(key),
                        escape_json_nul_characters(value),
                    )
                })
                .collect::<Map<_, _>>(),
        ),
        other => other,
    }
}

fn escape_nul_characters(text: String) -> String {
    if text.contains('\0') {
        text.replace('\0', "\\u0000")
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::escape_json_nul_characters;

    #[test]
    fn escapes_nul_characters_in_json_strings_and_keys() {
        let payload = json!({
            "tool\0key": [
                "before\0after",
                { "nested": "ok" }
            ]
        });

        assert_eq!(
            escape_json_nul_characters(payload),
            json!({
                "tool\\u0000key": [
                    "before\\u0000after",
                    { "nested": "ok" }
                ]
            })
        );
    }
}
