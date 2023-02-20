use anyhow::anyhow;
pub use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;

pub mod pretty;

/// Represents fields/content specified on the command line.
///
/// Sort of like HTTPie. Query parameters are '==', body values (JSON) are '='. Only single-level
/// body values are allowed currently, not JSON Pointer assignment.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ArgField {
    Query(String, serde_json::Value),
    Body(String, serde_json::Value),
}

impl FromStr for ArgField {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref FIELD_RE: Regex = Regex::new(r"^([a-zA-Z_]+)=(=)?(.*)$").unwrap();
        }
        if let Some(captures) = FIELD_RE.captures(s) {
            let key = captures[1].to_string();
            let val =
                Value::from_str(&captures[3]).unwrap_or(Value::String(captures[3].to_string()));
            let val = match val {
                Value::String(s) if s.is_empty() => Value::Null,
                _ => val,
            };
            if captures.get(2).is_some() {
                Ok(ArgField::Query(key, val))
            } else {
                Ok(ArgField::Body(key, val))
            }
        } else {
            Err(anyhow!("could not parse as a field assignment: {}", s))
        }
    }
}

#[test]
fn test_argfield() {
    use serde_json::json;
    assert_eq!(
        ArgField::from_str("a=3").unwrap(),
        ArgField::Body("a".to_string(), json!(3)),
    );
    assert_eq!(
        ArgField::from_str("a==3").unwrap(),
        ArgField::Query("a".to_string(), json!(3)),
    );
    assert_eq!(
        ArgField::from_str("cream==\"something\"").unwrap(),
        ArgField::Query("cream".to_string(), Value::String("something".to_string()))
    );
    assert_eq!(
        ArgField::from_str("cream==something").unwrap(),
        ArgField::Query("cream".to_string(), Value::String("something".to_string()))
    );
    assert_eq!(
        ArgField::from_str("cream=").unwrap(),
        ArgField::Body("cream".to_string(), Value::Null),
    );

    assert!(ArgField::from_str("a").is_err());
    assert!(ArgField::from_str("").is_err());
    assert!(ArgField::from_str("asdf.fee").is_err());

    assert!(ArgField::from_str("text=\"other value\"").is_ok());
}

// TODO: what should type signature actually be here...
pub fn update_params_from_fields(fields: &[ArgField], params: &mut HashMap<String, String>) {
    for f in fields.iter() {
        if let ArgField::Query(ref k, ref v) = f {
            match v {
                Value::String(s) => params.insert(k.to_string(), s.to_string()),
                _ => params.insert(k.to_string(), v.to_string()),
            };
        }
    }
}

pub fn update_value_from_fields(fields: Vec<ArgField>, value: &mut Value) {
    if let Value::Object(map) = value {
        for f in fields.into_iter() {
            if let ArgField::Body(k, v) = f {
                map.insert(k, v);
            }
        }
    }
}

/// Consumes the entire Vec of fields passed in
pub fn value_from_fields(fields: Vec<ArgField>) -> Value {
    let mut map: HashMap<String, Value> = HashMap::new();
    for f in fields.into_iter() {
        if let ArgField::Body(k, v) = f {
            map.insert(k, v);
        }
    }
    Value::Object(serde_json::map::Map::from_iter(map.into_iter()))
}
