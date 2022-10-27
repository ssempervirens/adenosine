use anyhow::anyhow;
pub use anyhow::Result;
use reqwest::header;
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

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
        body: Value,
    ) -> Result<Option<Value>> {
        let params: HashMap<String, String> = params.unwrap_or(HashMap::new());
        let res = self
            .http_client
            .post(format!("{}/xrpc/{}", self.host, nsid))
            .query(&params)
            .json(&body)
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

// TODO: parse at-uri
// at://did:plc:ltk4reuh7rkoy2frnueetpb5/app.bsky.follow/3jg23pbmlhc2a
