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
        nsid: String,
        params: Option<HashMap<String, String>>,
    ) -> Result<Option<Value>> {
        let params: HashMap<String, String> = params.unwrap_or(HashMap::new());
        let res = self
            .http_client
            .get(format!("{}/xrpc/{}", self.host, nsid))
            .query(&params)
            .send()?
            .error_for_status()?;
        Ok(res.json()?)
    }

    pub fn post(
        &self,
        nsid: String,
        params: Option<HashMap<String, String>>,
        body: Value,
    ) -> Result<Option<Value>> {
        let params: HashMap<String, String> = params.unwrap_or(HashMap::new());
        let res = self
            .http_client
            .get(format!("{}/xrpc/{}", self.host, nsid))
            .query(&params)
            .json(&body)
            .send()?
            .error_for_status()?;
        Ok(res.json()?)
    }
}
