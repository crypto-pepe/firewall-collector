use std::collections::HashMap;

use serde_json::{Map, Value};

use super::request;

pub fn headers(
    headers: HashMap<String, String>,
    headers_name_to_filter: Vec<String>,
) -> HashMap<String, String> {
    headers
        .into_iter()
        .filter(|(name, _)| !headers_name_to_filter.contains(name))
        .collect()
}

// The function deletes values by keys if the body is a json.
// In the case body is non-json, body is deleted if at least one of the keys is found by full-text search.
pub fn body(body: request::Body, keys: Vec<String>) -> request::Body {
    let data = match serde_json::from_str::<Map<String, Value>>(&body.data) {
        Ok(data) => {
            let mut ob = Value::Object(data);
            json(&mut ob, keys);
            ob.to_string()
        }
        Err(_) => match keys.into_iter().find(|k| body.data.contains(k)) {
            None => body.data,
            _ => "".to_string(),
        },
    };

    request::Body {
        data,
        state: body.state,
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

    use crate::service::{cleaner::json, request};

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
        let mut json_map =
            serde_json::json!( { "name": "John Doe", "age": 43, "some": {"age": 2, "other": 3} } );

        json(&mut json_map, Vec::from(["age".to_string()]));

        assert_eq!(json_map["name"].as_str().unwrap(), "John Doe");
        assert_eq!(json_map["some"]["other"], 3);
        assert_eq!(json_map["age"].is_string(), false);
        assert_eq!(json_map["some"]["age"].is_i64(), false);
    }

    #[test]
    fn body_without_found_key_in_json() {
        let raw = r#" { "name": "John Doe", "age": 43 } "#;

        let res = body(
            request::Body {
                data: raw.to_string(),
                ..body_init()
            },
            Vec::from(["age".to_string()]),
        );

        assert_eq!(r#"{"name":"John Doe"}"#, res.data);
    }

    #[test]
    fn empty_if_body_not_json() {
        let exp = r#" "name": "John Doe", "age": 43 } "#;

        let res = body(
            request::Body {
                data: exp.to_string(),
                ..body_init()
            },
            Vec::from(["age".to_string()]),
        );

        assert_eq!(res.data, "")
    }

    #[test]
    fn empty_body_if_found_key_in_not_json() {
        let exp = r#" : this not json "name" this not json  "#;

        let res = body(
            request::Body {
                data: exp.to_string(),
                ..body_init()
            },
            Vec::from(["name".to_string()]),
        );

        assert_eq!(res.data, "")
    }

    fn body_init() -> request::Body {
        request::Body {
            data: "some data".to_string(),
            state: request::BodyState::Original,
        }
    }
}
