use anyhow::{anyhow, Result};
use libipld::Ipld;
use log::{error, info};
use rouille::{router, Request, Response};
use serde_json::{json, Value};
use std::fmt;
use std::path::PathBuf;
use std::sync::Mutex;

mod car;
mod crypto;
mod db;
mod did;
mod models;
pub mod mst;
mod repo;

pub use car::{load_car_to_blockstore, load_car_to_sqlite};
pub use crypto::{KeyPair, PubKey};
pub use db::AtpDatabase;
pub use models::*;
pub use repo::{RepoCommit, RepoStore};

#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
struct AccountRequest {
    email: String,
    username: String,
    password: String,
    inviteCode: Option<String>,
    recoveryKey: Option<String>,
}

struct AtpService {
    pub repo: RepoStore,
    pub atp_db: AtpDatabase,
    pub pds_keypair: KeyPair,
    pub pds_public_url: String,
}

#[derive(Debug)]
enum XrpcError {
    BadRequest(String),
    NotFound(String),
    Forbidden(String),
}

impl std::error::Error for XrpcError {}

impl fmt::Display for XrpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadRequest(msg) | Self::NotFound(msg) | Self::Forbidden(msg) => {
                write!(f, "{}", msg)
            }
        }
    }
}

/// Helper to take an XRPC result (always a JSON object), and transform it to a rouille response
fn xrpc_wrap<S: serde::Serialize>(resp: Result<S>) -> Response {
    match resp {
        Ok(val) => Response::json(&val),
        Err(e) => {
            let msg = e.to_string();
            let code = match e.downcast_ref::<XrpcError>() {
                Some(XrpcError::BadRequest(_)) => 400,
                Some(XrpcError::NotFound(_)) => 404,
                Some(XrpcError::Forbidden(_)) => 403,
                None => 500,
            };
            Response::json(&json!({ "message": msg })).with_status_code(code)
        }
    }
}

pub fn run_server(port: u16, blockstore_db_path: &PathBuf, atp_db_path: &PathBuf) -> Result<()> {
    // TODO: some static files? https://github.com/tomaka/rouille/blob/master/examples/static-files.rs

    let srv = Mutex::new(AtpService {
        repo: RepoStore::open(blockstore_db_path)?,
        atp_db: AtpDatabase::open(atp_db_path)?,
        // XXX: reuse a keypair
        pds_keypair: KeyPair::new_random(),
        pds_public_url: format!("http://localhost:{}", port).to_string(),
    });

    let log_ok = |req: &Request, _resp: &Response, elap: std::time::Duration| {
        info!("{} {} ({:?})", req.method(), req.raw_url(), elap);
    };
    let log_err = |req: &Request, elap: std::time::Duration| {
        error!(
            "HTTP handler panicked: {} {} ({:?})",
            req.method(),
            req.raw_url(),
            elap
        );
    };
    rouille::start_server(format!("localhost:{}", port), move |request| {
        rouille::log_custom(request, log_ok, log_err, || {
            router!(request,
                (GET) ["/"] => {
                    Response::text("Not much to see here yet!")
                },
                (POST) ["/xrpc/com.atproto.{endpoint}", endpoint: String] => {
                    xrpc_wrap(xrpc_post_atproto(&srv, &endpoint, request))
                },
                (GET) ["/xrpc/com.atproto.{endpoint}", endpoint: String] => {
                    xrpc_wrap(xrpc_get_atproto(&srv, &endpoint, request))
                },
                _ => rouille::Response::empty_404()
            )
        })
    });
}

/// Intentionally serializing with this instead of DAG-JSON, because ATP schemas don't encode CID
/// links in any special way, they just pass the CID as a string.
fn ipld_into_json_value(val: Ipld) -> Value {
    match val {
        Ipld::Null => Value::Null,
        Ipld::Bool(b) => Value::Bool(b),
        Ipld::Integer(v) => json!(v),
        Ipld::Float(v) => json!(v),
        Ipld::String(s) => Value::String(s),
        Ipld::Bytes(b) => Value::String(data_encoding::BASE64_NOPAD.encode(&b)),
        Ipld::List(l) => Value::Array(l.into_iter().map(|v| ipld_into_json_value(v)).collect()),
        Ipld::Map(m) => Value::Object(serde_json::Map::from_iter(
            m.into_iter().map(|(k, v)| (k, ipld_into_json_value(v))),
        )),
        Ipld::Link(c) => Value::String(c.to_string()),
    }
}

fn xrpc_required_param(request: &Request, key: &str) -> Result<String> {
    Ok(request.get_param(key).ok_or(XrpcError::BadRequest(format!(
        "require '{}' query parameter",
        key
    )))?)
}

fn xrpc_get_atproto(
    srv: &Mutex<AtpService>,
    method: &str,
    request: &Request,
) -> Result<serde_json::Value> {
    match method {
        "getAccountsConfig" => {
            Ok(json!({"availableUserDomains": ["test"], "inviteCodeRequired": false}))
        }
        "getRecord" => {
            let did = xrpc_required_param(request, "did")?;
            let collection = xrpc_required_param(request, "collection")?;
            let rkey = xrpc_required_param(request, "rkey")?;
            let mut srv = srv.lock().expect("service mutex");
            let key = format!("/{}/{}", collection, rkey);
            match srv.repo.get_atp_record(&did, &collection, &rkey) {
                // TODO: format as JSON, not text debug
                Ok(Some(ipld)) => Ok(ipld_into_json_value(ipld)),
                Ok(None) => Err(anyhow!(XrpcError::NotFound(format!(
                    "could not find record: {}",
                    key
                )))),
                Err(e) => Err(e),
            }
        }
        "syncGetRoot" => {
            let did = xrpc_required_param(request, "did")?;
            let mut srv = srv.lock().expect("service mutex");
            srv.repo
                .lookup_commit(&did)?
                .map(|v| json!({ "root": v }))
                .ok_or(XrpcError::NotFound(format!("no repository found for DID: {}", did)).into())
        }
        _ => Err(anyhow!(XrpcError::NotFound(format!(
            "XRPC endpoint handler not found: com.atproto.{}",
            method
        )))),
    }
}

fn xrpc_post_atproto(
    srv: &Mutex<AtpService>,
    method: &str,
    request: &Request,
) -> Result<impl serde::Serialize> {
    match method {
        "createAccount" => {
            // TODO: generate did:plc, and insert an empty record/pointer to repo

            // validate account request
            let req: AccountRequest = rouille::input::json_input(request)
                .map_err(|e| XrpcError::BadRequest(format!("failed to parse JSON body: {}", e)))?;
            // TODO: validate username, email, recoverykey

            // check if account already exists (fast path, also confirmed by database schema)
            let mut srv = srv.lock().unwrap();
            if srv.atp_db.account_exists(&req.username, &req.email)? {
                Err(XrpcError::BadRequest(format!(
                    "username or email already exists"
                )))?;
            };

            // generate DID
            let create_op = did::CreateOp::new(
                req.username.clone(),
                srv.pds_public_url.clone(),
                &srv.pds_keypair,
                req.recoveryKey,
            );
            create_op.verify_self()?;
            let did = create_op.did_plc();
            let did_doc = create_op.did_doc();

            // register in ATP DB and generate DID doc
            srv.atp_db
                .create_account(&did, &req.username, &req.password, &req.email)?;
            srv.atp_db.put_did_doc(&did, &did_doc)?;

            // insert empty MST repository
            let root_cid = {
                let empty_map_cid: String = srv.repo.mst_from_map(&Default::default())?;
                let meta_cid = srv.repo.write_metadata(&did)?;
                srv.repo.write_root(&did, &meta_cid, None, &empty_map_cid)?
            };
            let _commit_cid = srv.repo.write_commit(&did, &root_cid, "XXX-dummy-sig")?;

            let sess = srv.atp_db.create_session(&req.username, &req.password)?;
            Ok(sess)
        }
        _ => Err(anyhow!(XrpcError::NotFound(format!(
            "XRPC endpoint handler not found: com.atproto.{}",
            method
        )))),
    }
}
