use adenosine_cli::identifiers::{Did, Nsid, Tid, TidLord};
use anyhow::Context;
use anyhow::{anyhow, Result};
use libipld::Cid;
use libipld::Ipld;
use log::{debug, error, info, warn};
use rouille::{router, Request, Response};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Mutex;

mod car;
mod crypto;
mod db;
mod did;
mod models;
pub mod mst;
mod repo;
mod ucan_p256;
mod vendored;

pub use car::{load_car_to_blockstore, load_car_to_sqlite};
pub use crypto::{KeyPair, PubKey};
pub use db::AtpDatabase;
pub use models::*;
pub use repo::{Mutation, RepoCommit, RepoStore};
pub use ucan_p256::P256KeyMaterial;

struct AtpService {
    pub repo: RepoStore,
    pub atp_db: AtpDatabase,
    pub pds_keypair: KeyPair,
    pub pds_public_url: String,
    pub tid_gen: TidLord,
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
            warn!("HTTP {}: {}", code, msg);
            Response::json(&json!({ "message": msg })).with_status_code(code)
        }
    }
}

pub fn run_server(
    port: u16,
    blockstore_db_path: &PathBuf,
    atp_db_path: &PathBuf,
    keypair: KeyPair,
) -> Result<()> {
    let srv = Mutex::new(AtpService {
        repo: RepoStore::open(blockstore_db_path)?,
        atp_db: AtpDatabase::open(atp_db_path)?,
        pds_keypair: keypair,
        pds_public_url: format!("http://localhost:{}", port),
        tid_gen: TidLord::new(),
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

    // TODO: robots.txt
    // TODO: some static files? https://github.com/tomaka/rouille/blob/master/examples/static-files.rs
    rouille::start_server(format!("localhost:{}", port), move |request| {
        rouille::log_custom(request, log_ok, log_err, || {
            router!(request,
                (GET) ["/"] => {
                    Response::text("Not much to see here yet!")
                },
                (POST) ["/xrpc/{endpoint}", endpoint: String] => {
                    xrpc_wrap(xrpc_post_handler(&srv, &endpoint, request))
                },
                (GET) ["/xrpc/{endpoint}", endpoint: String] => {
                    xrpc_wrap(xrpc_get_handler(&srv, &endpoint, request))
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
        Ipld::List(l) => Value::Array(l.into_iter().map(ipld_into_json_value).collect()),
        Ipld::Map(m) => Value::Object(serde_json::Map::from_iter(
            m.into_iter().map(|(k, v)| (k, ipld_into_json_value(v))),
        )),
        Ipld::Link(c) => Value::String(c.to_string()),
    }
}

/// Crude reverse generation
///
/// Does not handle base64 to bytes, and the link generation is pretty simple (object elements with
/// key "car"). Numbers always come through as f64 (float).
fn json_value_into_ipld(val: Value) -> Ipld {
    match val {
        Value::Null => Ipld::Null,
        Value::Bool(b) => Ipld::Bool(b),
        Value::String(s) => Ipld::String(s),
        // TODO: handle numbers better?
        Value::Number(v) => Ipld::Float(v.as_f64().unwrap()),
        Value::Array(l) => Ipld::List(l.into_iter().map(json_value_into_ipld).collect()),
        Value::Object(m) => {
            let map: BTreeMap<String, Ipld> = BTreeMap::from_iter(m.into_iter().map(|(k, v)| {
                if k == "car" && v.is_string() {
                    (k, Ipld::Link(Cid::from_str(v.as_str().unwrap()).unwrap()))
                } else {
                    (k, json_value_into_ipld(v))
                }
            }));
            Ipld::Map(map)
        }
    }
}

fn xrpc_required_param(request: &Request, key: &str) -> Result<String> {
    Ok(request.get_param(key).ok_or(XrpcError::BadRequest(format!(
        "require '{}' query parameter",
        key
    )))?)
}

/// Returns DID of validated user
fn xrpc_check_auth_header(
    srv: &mut AtpService,
    request: &Request,
    req_did: Option<&Did>,
) -> Result<Did> {
    let header = request
        .header("Authorization")
        .ok_or(XrpcError::Forbidden("require auth header".to_string()))?;
    if !header.starts_with("Bearer ") {
        Err(XrpcError::Forbidden("require bearer token".to_string()))?;
    }
    let jwt = header.split(' ').nth(1).unwrap();
    let did = match srv.atp_db.check_auth_token(jwt)? {
        Some(did) => did,
        None => Err(XrpcError::Forbidden("session token not found".to_string()))?,
    };
    let did = Did::from_str(&did)?;
    if req_did.is_some() && Some(&did) != req_did {
        Err(XrpcError::Forbidden(
            "can only modify your own repo".to_string(),
        ))?;
    }
    Ok(did)
}

fn xrpc_get_handler(
    srv: &Mutex<AtpService>,
    method: &str,
    request: &Request,
) -> Result<serde_json::Value> {
    match method {
        "com.atproto.getAccountsConfig" => {
            Ok(json!({"availableUserDomains": ["test"], "inviteCodeRequired": false}))
        }
        "com.atproto.repoGetRecord" => {
            let did = Did::from_str(&xrpc_required_param(request, "user")?)?;
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
        "com.atproto.syncGetRoot" => {
            let did = Did::from_str(&xrpc_required_param(request, "did")?)?;
            let mut srv = srv.lock().expect("service mutex");
            srv.repo
                .lookup_commit(&did)?
                .map(|v| json!({ "root": v }))
                .ok_or(XrpcError::NotFound(format!("no repository found for DID: {}", did)).into())
        }
        "com.atproto.repoListRecords" => {
            // TODO: limit, before, after, tid, reverse
            // TODO: handle non-DID 'user'
            // TODO: limit result set size
            let did = Did::from_str(&xrpc_required_param(request, "user")?)?;
            let collection = Nsid::from_str(&xrpc_required_param(request, "collection")?)?;
            let mut record_list: Vec<Value> = vec![];
            let mut srv = srv.lock().expect("service mutex");
            let commit_cid = &srv.repo.lookup_commit(&did)?.unwrap();
            let last_commit = srv.repo.get_commit(commit_cid)?;
            let full_map = srv.repo.mst_to_map(&last_commit.mst_cid)?;
            let prefix = format!("/{}/", collection);
            for (mst_key, cid) in full_map.iter() {
                debug!("{}", mst_key);
                if mst_key.starts_with(&prefix) {
                    let record = srv.repo.get_ipld(cid)?;
                    record_list.push(json!({
                        "uri": format!("at://{}{}", did, mst_key),
                        "cid": cid,
                        "value": ipld_into_json_value(record),
                    }));
                }
            }
            Ok(json!({ "records": record_list }))
        }
        "com.atproto.repoDescribe" => {
            let did = Did::from_str(&xrpc_required_param(request, "user")?)?;
            // TODO: resolve username?
            let username = did.to_string();
            let mut srv = srv.lock().expect("service mutex");
            let did_doc = srv.atp_db.get_did_doc(&did)?;
            let collections: Vec<String> = srv.repo.collections(&did)?;
            let desc = RepoDescribe {
                name: username,
                did: did.to_string(),
                didDoc: did_doc,
                collections: collections,
                nameIsCorrect: true,
            };
            Ok(json!(desc))
        }
        _ => Err(anyhow!(XrpcError::NotFound(format!(
            "XRPC endpoint handler not found: {}",
            method
        )))),
    }
}

fn xrpc_post_handler(
    srv: &Mutex<AtpService>,
    method: &str,
    request: &Request,
) -> Result<serde_json::Value> {
    match method {
        "com.atproto.createAccount" => {
            // validate account request
            let req: AccountRequest = rouille::input::json_input(request)
                .map_err(|e| XrpcError::BadRequest(format!("failed to parse JSON body: {}", e)))?;
            // TODO: validate username, email, recoverykey

            // check if account already exists (fast path, also confirmed by database schema)
            let mut srv = srv.lock().unwrap();
            if srv.atp_db.account_exists(&req.username, &req.email)? {
                Err(XrpcError::BadRequest(
                    "username or email already exists".to_string(),
                ))?;
            };

            debug!("trying to create new account: {}", &req.username);

            // generate DID
            let create_op = did::CreateOp::new(
                req.username.clone(),
                srv.pds_public_url.clone(),
                &srv.pds_keypair,
                req.recoveryKey.clone(),
            );
            create_op.verify_self()?;
            let did = create_op.did_plc();
            let did_doc = create_op.did_doc();

            // register in ATP DB and generate DID doc
            let recovery_key = req
                .recoveryKey
                .unwrap_or(srv.pds_keypair.pubkey().to_did_key());
            srv.atp_db.create_account(
                &did,
                &req.username,
                &req.password,
                &req.email,
                &recovery_key,
            )?;
            srv.atp_db.put_did_doc(&did, &did_doc)?;

            // insert empty MST repository
            let root_cid = {
                let empty_map_cid: String = srv.repo.mst_from_map(&Default::default())?;
                let meta_cid = srv.repo.write_metadata(&did)?;
                srv.repo.write_root(&meta_cid, None, &empty_map_cid)?
            };
            let _commit_cid = srv.repo.write_commit(&did, &root_cid, "XXX-dummy-sig")?;

            let keypair = srv.pds_keypair.clone();
            let sess = srv
                .atp_db
                .create_session(&req.username, &req.password, &keypair)?;
            Ok(json!(sess))
        }
        "com.atproto.createSession" => {
            let req: SessionRequest = rouille::input::json_input(request)
                .map_err(|e| XrpcError::BadRequest(format!("failed to parse JSON body: {}", e)))?;
            let mut srv = srv.lock().unwrap();
            let keypair = srv.pds_keypair.clone();
            Ok(json!(srv.atp_db.create_session(
                &req.username,
                &req.password,
                &keypair
            )?))
        }
        "com.atproto.deleteSession" => {
            let mut srv = srv.lock().unwrap();
            let _did = xrpc_check_auth_header(&mut srv, request, None)?;
            let header = request
                .header("Authorization")
                .ok_or(XrpcError::Forbidden("require auth header".to_string()))?;
            if !header.starts_with("Bearer ") {
                Err(XrpcError::Forbidden("require bearer token".to_string()))?;
            }
            let jwt = header.split(' ').nth(1).expect("JWT in header");
            if !srv.atp_db.delete_session(jwt)? {
                Err(anyhow!(
                    "session token not found, even after using for auth"
                ))?
            };
            Ok(json!({}))
        }
        "com.atproto.repoBatchWrite" => {
            let batch: RepoBatchWriteBody = rouille::input::json_input(request)?;
            // TODO: validate edits against schemas
            let did = Did::from_str(&xrpc_required_param(request, "did")?)?;
            let mut srv = srv.lock().unwrap();
            let _auth_did = &xrpc_check_auth_header(&mut srv, request, Some(&did))?;
            let commit_cid = &srv.repo.lookup_commit(&did)?.unwrap();
            let last_commit = srv.repo.get_commit(commit_cid)?;
            let mut mutations: Vec<Mutation> = Default::default();
            for w in batch.writes.iter() {
                let m = match w.op_type.as_str() {
                    "create" => Mutation::Create(
                        Nsid::from_str(&w.collection)?,
                        // TODO: user input unwrap here
                        w.rkey
                            .as_ref()
                            .map(|t| Tid::from_str(t).unwrap())
                            .unwrap_or_else(|| srv.tid_gen.next_tid()),
                        json_value_into_ipld(w.value.clone()),
                    ),
                    "update" => Mutation::Update(
                        Nsid::from_str(&w.collection)?,
                        Tid::from_str(w.rkey.as_ref().unwrap())?,
                        json_value_into_ipld(w.value.clone()),
                    ),
                    "delete" => Mutation::Delete(
                        Nsid::from_str(&w.collection)?,
                        Tid::from_str(w.rkey.as_ref().unwrap())?,
                    ),
                    _ => Err(anyhow!("unhandled operation type: {}", w.op_type))?,
                };
                mutations.push(m);
            }
            let new_mst_cid = srv.repo.update_mst(&last_commit.mst_cid, &mutations)?;
            let new_root_cid = srv.repo.write_root(
                &last_commit.meta_cid,
                Some(&last_commit.commit_cid),
                &new_mst_cid,
            )?;
            srv.repo.write_commit(&did, &new_root_cid, "dummy-sig")?;
            // TODO: next handle updates to database
            Ok(json!({}))
        }
        "com.atproto.repoCreateRecord" => {
            // TODO: validate edits against schemas
            let did = Did::from_str(&xrpc_required_param(request, "did")?)?;
            let collection = Nsid::from_str(&xrpc_required_param(request, "collection")?)?;
            let record: Value = rouille::input::json_input(request)?;
            let mut srv = srv.lock().unwrap();
            let _auth_did = &xrpc_check_auth_header(&mut srv, request, Some(&did))?;
            debug!("reading commit");
            let commit_cid = &srv.repo.lookup_commit(&did)?.unwrap();
            let last_commit = srv.repo.get_commit(commit_cid)?;
            let mutations: Vec<Mutation> = vec![Mutation::Create(
                collection,
                srv.tid_gen.next_tid(),
                json_value_into_ipld(record),
            )];
            debug!("mutating tree");
            let new_mst_cid = srv
                .repo
                .update_mst(&last_commit.mst_cid, &mutations)
                .context("updating MST in repo")?;
            debug!("writing new root");
            let new_root_cid = srv.repo.write_root(
                &last_commit.meta_cid,
                Some(&last_commit.commit_cid),
                &new_mst_cid,
            )?;
            debug!("writing new commit");
            srv.repo.write_commit(&did, &new_root_cid, "dummy-sig")?;
            // TODO: next handle updates to database
            Ok(json!({}))
        }
        "com.atproto.repoPutRecord" => {
            // TODO: validate edits against schemas
            let did = Did::from_str(&xrpc_required_param(request, "did")?)?;
            let collection = Nsid::from_str(&xrpc_required_param(request, "collection")?)?;
            let tid = Tid::from_str(&xrpc_required_param(request, "rkey")?)?;
            let record: Value = rouille::input::json_input(request)?;
            let mut srv = srv.lock().unwrap();
            let _auth_did = &xrpc_check_auth_header(&mut srv, request, Some(&did))?;
            let commit_cid = &srv.repo.lookup_commit(&did)?.unwrap();
            let last_commit = srv.repo.get_commit(commit_cid)?;
            let mutations: Vec<Mutation> = vec![Mutation::Update(
                collection,
                tid,
                json_value_into_ipld(record),
            )];
            let new_mst_cid = srv
                .repo
                .update_mst(&last_commit.mst_cid, &mutations)
                .context("updating MST in repo")?;
            let new_root_cid = srv.repo.write_root(
                &last_commit.meta_cid,
                Some(&last_commit.commit_cid),
                &new_mst_cid,
            )?;
            srv.repo.write_commit(&did, &new_root_cid, "dummy-sig")?;
            // TODO: next handle updates to database
            Ok(json!({}))
        }
        "com.atproto.repoDeleteRecord" => {
            let did = Did::from_str(&xrpc_required_param(request, "did")?)?;
            let collection = Nsid::from_str(&xrpc_required_param(request, "collection")?)?;
            let tid = Tid::from_str(&xrpc_required_param(request, "rkey")?)?;
            let mut srv = srv.lock().unwrap();
            let _auth_did = &xrpc_check_auth_header(&mut srv, request, Some(&did))?;
            let commit_cid = &srv.repo.lookup_commit(&did)?.unwrap();
            let last_commit = srv.repo.get_commit(commit_cid)?;
            let mutations: Vec<Mutation> = vec![Mutation::Delete(collection, tid)];
            let new_mst_cid = srv.repo.update_mst(&last_commit.mst_cid, &mutations)?;
            let new_root_cid = srv.repo.write_root(
                &last_commit.meta_cid,
                Some(&last_commit.commit_cid),
                &new_mst_cid,
            )?;
            srv.repo.write_commit(&did, &new_root_cid, "dummy-sig")?;
            // TODO: next handle updates to database
            Ok(json!({}))
        }
        _ => Err(anyhow!(XrpcError::NotFound(format!(
            "XRPC endpoint handler not found: {}",
            method
        )))),
    }
}
