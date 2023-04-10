use crate::auth::parse_did_from_jwt;
use crate::identifiers::{Did, Nsid};
use anyhow::anyhow;
pub use anyhow::Result;
use base64;
use log::warn;
use reqwest::header;
use serde_json::{json, Value};
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

#[derive(Debug, Clone)]
pub struct XrpcClient {
    http_client: reqwest::blocking::Client,
    host: String,
    auth_token: Option<String>,
    refresh_token: Option<String>,
    admin_password: Option<String>,
}

impl XrpcClient {
    pub fn new(
        host: String,
        auth_token: Option<String>,
        admin_password: Option<String>,
    ) -> Result<Self> {
        let http_client = reqwest::blocking::Client::builder()
            .user_agent(APP_USER_AGENT)
            .timeout(Duration::from_secs(30))
            //.danger_accept_invalid_certs(true)
            .build()
            .expect("ERROR :: Could not build reqwest client");

        Ok(XrpcClient {
            http_client,
            host,
            auth_token: auth_token.clone(),
            refresh_token: auth_token,
            admin_password,
        })
    }

    fn auth_headers(&self, endpoint: &str) -> reqwest::header::HeaderMap {
        let mut headers = header::HeaderMap::new();
        if endpoint == "com.atproto.account.createInviteCode"
            || endpoint.starts_with("com.atproto.admin.")
        {
            if let Some(admin_password) = &self.admin_password {
                let enc =
                    base64::encode_config(format!("admin:{admin_password}"), base64::STANDARD);
                let mut auth_value = header::HeaderValue::from_str(&format!("Basic {enc}"))
                    .expect("header formatting");
                auth_value.set_sensitive(true);
                headers.insert(header::AUTHORIZATION, auth_value);
                return headers;
            } else {
                warn!("endpoint requires admin auth, but password not supplied: {endpoint}")
            };
        };
        if let Some(token) = &self.auth_token {
            let mut auth_value = header::HeaderValue::from_str(&format!("Bearer {token}"))
                .expect("header formatting");
            auth_value.set_sensitive(true);
            headers.insert(header::AUTHORIZATION, auth_value);
        };
        headers
    }

    /// Creates a new session, and updates current client auth tokens with the result
    pub fn auth_login(&mut self, identifier: &str, password: &str) -> Result<()> {
        let resp = self.post(
            &Nsid::from_str("com.atproto.server.createSession")?,
            None,
            Some(json!({
                "identifier": identifier,
                "password": password,
            })),
        )?;
        let resp = resp.ok_or(anyhow!("missing session auth info"))?;
        self.auth_token = resp["accessJwt"].as_str().map(|s| s.to_string());
        self.refresh_token = resp["refreshJwt"].as_str().map(|s| s.to_string());
        Ok(())
    }

    /// Uses refresh token to update auth token
    pub fn auth_refresh(&mut self) -> Result<()> {
        self.auth_token = self.refresh_token.clone();
        let resp = self.post(&Nsid::from_str("com.atproto.session.refresh")?, None, None)?;
        let resp = resp.ok_or(anyhow!("missing session auth info"))?;
        self.auth_token = resp["accessJwt"].as_str().map(|s| s.to_string());
        self.refresh_token = resp["refreshJwt"].as_str().map(|s| s.to_string());
        Ok(())
    }

    pub fn auth_did(&self) -> Result<Did> {
        if let Some(token) = &self.auth_token {
            Did::from_str(&parse_did_from_jwt(token)?)
        } else {
            Err(anyhow!("no auth token configured"))
        }
    }

    pub fn get(
        &self,
        nsid: &Nsid,
        params: Option<HashMap<String, String>>,
    ) -> Result<Option<Value>> {
        log::debug!("XRPC GET endpoint={} params={:?}", nsid, params);
        let params: HashMap<String, String> = params.unwrap_or_default();
        let res = self
            .http_client
            .get(format!("{}/xrpc/{nsid}", self.host))
            .headers(self.auth_headers(nsid))
            .query(&params)
            .send()?;
        // TODO: refactor this error handling stuff into single method
        if res.status() == 400 {
            let val: Value = res.json()?;
            return Err(anyhow!(
                "XRPC Bad Request (400): {}",
                val["message"].as_str().unwrap_or("unknown")
            ));
        } else if res.status() == 500 {
            let val: Value = res.json()?;
            return Err(anyhow!(
                "XRPC Internal Error (500): {}",
                val["message"].as_str().unwrap_or("unknown")
            ));
        }
        let res = res.error_for_status()?;
        Ok(res.json()?)
    }

    pub fn get_to_writer<W: std::io::Write>(
        &self,
        nsid: &Nsid,
        params: Option<HashMap<String, String>>,
        output: &mut W,
    ) -> Result<u64> {
        let params: HashMap<String, String> = params.unwrap_or_default();
        let res = self
            .http_client
            .get(format!("{}/xrpc/{}", self.host, nsid))
            .headers(self.auth_headers(nsid))
            .query(&params)
            .send()?;
        if res.status() == 400 {
            let val: Value = res.json()?;
            return Err(anyhow!(
                "XRPC Bad Request (400): {}",
                val["message"].as_str().unwrap_or("unknown")
            ));
        } else if res.status() == 500 {
            let val: Value = res.json()?;
            return Err(anyhow!(
                "XRPC Internal Error (500): {}",
                val["message"].as_str().unwrap_or("unknown")
            ));
        }
        let mut res = res.error_for_status()?;
        Ok(res.copy_to(output)?)
    }

    pub fn post(
        &self,
        nsid: &Nsid,
        params: Option<HashMap<String, String>>,
        body: Option<Value>,
    ) -> Result<Option<Value>> {
        let params: HashMap<String, String> = params.unwrap_or_default();
        log::debug!(
            "XRPC POST endpoint={} params={:?} body={:?}",
            nsid,
            params,
            body
        );
        let mut req = self
            .http_client
            .post(format!("{}/xrpc/{}", self.host, nsid))
            .headers(self.auth_headers(nsid))
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
                "XRPC Bad Request (400): {}",
                val["message"].as_str().unwrap_or("unknown")
            ));
        } else if res.status() == 500 {
            let val: Value = res.json()?;
            return Err(anyhow!(
                "XRPC Internal Error (500): {}",
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
        nsid: &Nsid,
        params: Option<HashMap<String, String>>,
        input: &mut R,
    ) -> Result<Option<Value>> {
        let params: HashMap<String, String> = params.unwrap_or_default();
        let mut buf: Vec<u8> = Vec::new();
        input.read_to_end(&mut buf)?;
        let res = self
            .http_client
            .post(format!("{}/xrpc/{}", self.host, nsid))
            .headers(self.auth_headers(nsid))
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
}
