use anyhow::anyhow;
pub use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::header;
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

mod identifiers;
pub use identifiers::{AtUri, DidOrHost};

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum XrpcMethod {
    Get,
    Post,
}

impl FromStr for XrpcMethod {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "get" => Ok(XrpcMethod::Get),
            "post" => Ok(XrpcMethod::Post),
            _ => Err(anyhow!("unknown method: {}", s)),
        }
    }
}

pub struct XrpcClient {
    http_client: reqwest::blocking::Client,
    host: String,
}

impl XrpcClient {
    pub fn new(host: String, auth_token: Option<String>) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        if let Some(token) = &auth_token {
            let mut auth_value = header::HeaderValue::from_str(&format!("Bearer {}", token))?;
            auth_value.set_sensitive(true);
            headers.insert(header::AUTHORIZATION, auth_value);
        };

        let http_client = reqwest::blocking::Client::builder()
            .default_headers(headers)
            .user_agent(APP_USER_AGENT)
            .timeout(Duration::from_secs(30))
            //.danger_accept_invalid_certs(true)
            .build()
            .expect("ERROR :: Could not build reqwest client");

        Ok(XrpcClient { http_client, host })
    }

    pub fn get(
        &self,
        nsid: &str,
        params: Option<HashMap<String, String>>,
    ) -> Result<Option<Value>> {
        let params: HashMap<String, String> = params.unwrap_or(HashMap::new());
        let res = self
            .http_client
            .get(format!("{}/xrpc/{}", self.host, nsid))
            .query(&params)
            .send()?;
        if res.status() == 400 {
            let val: Value = res.json()?;
            return Err(anyhow!(
                "XRPC Bad Request: {}",
                val["message"].as_str().unwrap_or("unknown")
            ));
        }
        let res = res.error_for_status()?;
        Ok(res.json()?)
    }

    pub fn get_to_writer<W: std::io::Write>(
        &self,
        nsid: &str,
        params: Option<HashMap<String, String>>,
        output: &mut W,
    ) -> Result<u64> {
        let params: HashMap<String, String> = params.unwrap_or(HashMap::new());
        let res = self
            .http_client
            .get(format!("{}/xrpc/{}", self.host, nsid))
            .query(&params)
            .send()?;
        if res.status() == 400 {
            let val: Value = res.json()?;
            return Err(anyhow!(
                "XRPC Bad Request: {}",
                val["message"].as_str().unwrap_or("unknown")
            ));
        }
        let mut res = res.error_for_status()?;
        Ok(res.copy_to(output)?)
    }

    pub fn post(
        &self,
        nsid: &str,
        params: Option<HashMap<String, String>>,
        body: Option<Value>,
    ) -> Result<Option<Value>> {
        let params: HashMap<String, String> = params.unwrap_or(HashMap::new());
        let mut req = self
            .http_client
            .post(format!("{}/xrpc/{}", self.host, nsid))
            .query(&params);
        req = if let Some(b) = body {
            req.json(&b)
        } else {
            req
        };
        let res = req.send()?;
        if res.status() == 400 {
            let val: Value = res.json()?;
            return Err(anyhow!(
                "XRPC Bad Request: {}",
                val["message"].as_str().unwrap_or("unknown")
            ));
        }
        let res = res.error_for_status()?;
        if res.content_length() == Some(0) {
            Ok(None)
        } else {
            Ok(res.json()?)
        }
    }

    pub fn post_cbor_from_reader<R: std::io::Read>(
        &self,
        nsid: &str,
        params: Option<HashMap<String, String>>,
        input: &mut R,
    ) -> Result<Option<Value>> {
        let params: HashMap<String, String> = params.unwrap_or(HashMap::new());
        let mut buf: Vec<u8> = Vec::new();
        input.read_to_end(&mut buf)?;
        let res = self
            .http_client
            .post(format!("{}/xrpc/{}", self.host, nsid))
            .query(&params)
            .header(reqwest::header::CONTENT_TYPE, "application/cbor")
            .body(buf)
            .send()?;
        if res.status() == 400 {
            let val: Value = res.json()?;
            return Err(anyhow!(
                "XRPC Bad Request: {}",
                val["message"].as_str().unwrap_or("unknown")
            ));
        }
        let res = res.error_for_status()?;
        Ok(res.json()?)
    }

    //  reqwest::blocking::Body
}

/// Tries to parse a DID internal identifier from a JWT (as base64-encoded token)
pub fn parse_did_from_jwt(jwt: &str) -> Result<String> {
    let second_b64 = jwt.split(".").nth(1).ok_or(anyhow!("couldn't parse JWT"))?;
    let second_json: Vec<u8> = base64::decode_config(second_b64, base64::URL_SAFE)?;
    let obj: Value = serde_json::from_slice(&second_json)?;
    let did = obj["sub"]
        .as_str()
        .ok_or(anyhow!("couldn't find DID subject in JWT"))?
        .to_string();
    if !did.starts_with("did:") {
        return Err(anyhow!("couldn't find DID subject in JWT"));
    }
    Ok(did)
}

#[test]
fn test_parse_jwt() {
    assert!(parse_did_from_jwt(".").is_err());
    assert_eq!(
        parse_did_from_jwt("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkaWQ6cGxjOmV4M3NpNTI3Y2QyYW9nYnZpZGtvb296YyIsImlhdCI6MTY2NjgyOTM5M30.UvZgTqvaJICONa1wIUT1bny7u3hqVAqWhWy3qeuyZrE").unwrap(),
        "did:plc:ex3si527cd2aogbvidkooozc",
    );
    assert!(parse_did_from_jwt("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9").is_err());
}

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
            params.insert(k.to_string(), v.to_string());
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