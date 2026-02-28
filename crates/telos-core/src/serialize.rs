use crate::error::CoreError;
use crate::hash::ObjectId;
use serde::Serialize;

/// Produce a canonical byte representation: `type_tag\0sorted_json`.
///
/// The JSON keys are sorted to ensure deterministic output regardless
/// of field declaration or insertion order.
pub fn canonical_serialize(type_tag: &str, value: &impl Serialize) -> Result<Vec<u8>, CoreError> {
    let json_value = serde_json::to_value(value)?;
    let sorted_json = serde_json::to_string(&sort_value(json_value))?;
    let mut buf = Vec::with_capacity(type_tag.len() + 1 + sorted_json.len());
    buf.extend_from_slice(type_tag.as_bytes());
    buf.push(0); // null separator
    buf.extend_from_slice(sorted_json.as_bytes());
    Ok(buf)
}

/// Compute the ObjectId for a typed, serializable value.
pub fn content_hash(type_tag: &str, value: &impl Serialize) -> Result<ObjectId, CoreError> {
    let bytes = canonical_serialize(type_tag, value)?;
    Ok(ObjectId::hash(&bytes))
}

/// Recursively sort all object keys in a JSON value.
fn sort_value(v: serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(map) => {
            let sorted: serde_json::Map<String, serde_json::Value> = map
                .into_iter()
                .map(|(k, v)| (k, sort_value(v)))
                .collect::<std::collections::BTreeMap<_, _>>()
                .into_iter()
                .collect();
            serde_json::Value::Object(sorted)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(sort_value).collect())
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Serialize, Deserialize)]
    struct Sample {
        zebra: String,
        alpha: i32,
    }

    #[test]
    fn canonical_serialize_sorted_keys() {
        let s = Sample {
            zebra: "z".into(),
            alpha: 1,
        };
        let bytes = canonical_serialize("sample", &s).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.starts_with("sample\0"));
        let json_part = &text["sample\0".len()..];
        // "alpha" must come before "zebra"
        let alpha_pos = json_part.find("\"alpha\"").unwrap();
        let zebra_pos = json_part.find("\"zebra\"").unwrap();
        assert!(alpha_pos < zebra_pos);
    }

    #[test]
    fn content_hash_deterministic() {
        let s = Sample {
            zebra: "z".into(),
            alpha: 1,
        };
        let h1 = content_hash("sample", &s).unwrap();
        let h2 = content_hash("sample", &s).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn different_type_tags_different_hash() {
        let s = Sample {
            zebra: "z".into(),
            alpha: 1,
        };
        let h1 = content_hash("sample", &s).unwrap();
        let h2 = content_hash("other", &s).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn nested_objects_sorted() {
        let mut map = HashMap::new();
        map.insert("z_key".to_string(), "val");
        map.insert("a_key".to_string(), "val");
        let bytes = canonical_serialize("test", &map).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let json_part = &text["test\0".len()..];
        let a_pos = json_part.find("\"a_key\"").unwrap();
        let z_pos = json_part.find("\"z_key\"").unwrap();
        assert!(a_pos < z_pos);
    }

    #[test]
    fn unicode_content() {
        let s = Sample {
            zebra: "你好世界🌍".into(),
            alpha: 42,
        };
        let h = content_hash("sample", &s).unwrap();
        assert_eq!(h.hex().len(), 64);
    }
}
