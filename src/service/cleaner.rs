use std::collections::HashMap;

use serde_json::{Map, Value};

pub fn headers(
    headers: HashMap<String, String>,
    names_headers: Vec<String>,
) -> HashMap<String, String> {
    headers
        .into_iter()
        .filter(|(name, _)| !names_headers.contains(name))
        .collect()
}

pub fn body(body: String, keys: Vec<String>) -> String {
    match serde_json::from_str::<Map<String, Value>>(&body) {
        Ok(data) => {
            let mut ob = Value::Object(data);
            json(&mut ob, keys);
            ob.to_string()
        }
        Err(_) => match keys.into_iter().find(|k| body.contains(k)) {
            None => body,
            _ => "".to_string(),
        },
    }
}

fn json(value: &mut Value, keys: Vec<String>) {
    if let Value::Object(map) = value {
        map.retain(|k, _| !keys.contains(k));
        map.iter_mut().map(|(_, v)| json(v, keys.clone())).collect()
    };
}

#[cfg(test)]
mod cleaner_test {
    use std::collections::HashMap;

    use serde_json::{Map, Value};

    use crate::service::cleaner::json;

    use super::{body, headers};

    #[test]
    fn clean_headers() {
        let res = headers(
            HashMap::from([
                (String::from("header1"), String::from("header value")),
                (String::from("header2"), String::from("header_value")),
                (String::from("header3"), String::from("header_value")),
            ]),
            Vec::from(["header1".to_string(), "header3".to_string()]),
        );

        assert_eq!(
            res,
            HashMap::from([(String::from("header2"), String::from("header_value"))]),
        )
    }

    #[test]
    fn json_test() {
        let raw = r#" { "name": "John Doe", "age": 43, "some": {"age": 2, "other": 3} } "#;
        let mut json_map = Value::Object(serde_json::from_str::<Map<String, Value>>(&raw).unwrap());

        json(&mut json_map, Vec::from(["age".to_string()]));

        assert_eq!(json_map["name"].as_str().unwrap(), "John Doe");
        assert_eq!(json_map["some"]["other"], 3);
        assert_eq!(json_map["age"].is_string(), false);
        assert_eq!(json_map["some"]["age"].is_i64(), false);
    }

    #[test]
    fn empty_if_body_not_json() {
        let exp = r#" "name": "John Doe", "age": 43 } "#;

        let res = body(exp.to_string(), Vec::from(["age".to_string()]));

        assert_eq!(res, "")
    }

    #[test]
    fn body_without_key_if_json() {
        let raw = r#" { "name": "John Doe", "age": 43 } "#;

        let res = body(raw.to_string(), Vec::from(["age".to_string()]));

        assert_eq!(r#"{"name":"John Doe"}"#, res);
    }

    #[test]
    fn not_empty_if_not_json_is_find() {
        let exp = r#" "name": "John Doe", "age": 43 } "#;

        let res = body(exp.to_string(), Vec::from(["name".to_string()]));

        assert_eq!(res, "")
    }
}
