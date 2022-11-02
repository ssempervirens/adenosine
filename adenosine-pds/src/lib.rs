use anyhow::{anyhow, Result};
use log::{error, info};
use rouille::{router, try_or_400, Request, Response};
use serde_json::json;
use std::fmt;
use std::path::PathBuf;
use std::sync::Mutex;

use ipfs_sqlite_block_store::BlockStore;

mod car;
mod db;
mod models;
pub mod mst;
mod repo;

pub use car::{load_car_to_blockstore, load_car_to_sqlite};
pub use db::AtpDatabase;
pub use models::*;
pub use repo::{RepoCommit, RepoStore};

#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
struct AccountRequest {
    email: String,
    username: String,
    password: String,
}

struct AtpService {
    pub repo: RepoStore,
    pub atp_db: AtpDatabase,
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
                (POST) ["/xrpc/com.atproto.createAccount"] => {
                    let req: AccountRequest = try_or_400!(rouille::input::json_input(request));
                    let mut srv = srv.lock().unwrap();
                    xrpc_wrap(srv.atp_db.create_account(&req.username, &req.password, &req.email))
                },
                (GET) ["/xrpc/com.atproto.{endpoint}", endpoint: String] => {
                    xrpc_wrap(xrpc_get_atproto(&srv, &endpoint, request))
                },
                _ => rouille::Response::empty_404()
            )
        })
    });
}

fn xrpc_get_atproto(
    srv: &Mutex<AtpService>,
    method: &str,
    request: &Request,
) -> Result<serde_json::Value> {
    match method {
        "getRecord" => {
            let did = request.get_param("user").unwrap();
            let collection = request.get_param("collection").unwrap();
            let rkey = request.get_param("rkey").unwrap();
            let repo_key = format!("/{}/{}", collection, rkey);
            let mut srv = srv.lock().expect("service mutex");
            let commit_cid = srv.repo.lookup_commit(&did)?.unwrap();
            let key = format!("/{}/{}", collection, rkey);
            match srv.repo.get_record_by_key(&commit_cid, &key) {
                // TODO: format as JSON, not text debug
                Ok(Some(ipld)) => Ok(json!({ "thing": format!("{:?}", ipld) })),
                Ok(None) => Err(anyhow!(XrpcError::NotFound(format!(
                    "could not find record: {}",
                    key
                )))),
                Err(e) => Err(e),
            }
        }
        "syncGetRoot" => {
            let did = request.get_param("did").unwrap();
            let mut srv = srv.lock().expect("service mutex");
            srv.repo
                .lookup_commit(&did)?
                .map(|v| json!({ "root": v }))
                .ok_or(anyhow!("XXX: missing"))
        }
        _ => Err(anyhow!(XrpcError::NotFound(format!(
            "XRPC endpoint handler not found: com.atproto.{}",
            method
        )))),
    }
}
