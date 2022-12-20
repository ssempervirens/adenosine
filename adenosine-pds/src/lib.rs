use adenosine_cli::created_at_now;
use adenosine_cli::identifiers::{AtUri, Did, Nsid, Ticker, Tid};
use anyhow::{anyhow, Result};
use askama::Template;
use libipld::Cid;
use libipld::Ipld;
use log::{debug, error, info, warn};
use rouille::{router, Request, Response};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fmt;
use std::io::Read;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Mutex;

mod bsky;
mod car;
mod crypto;
mod db;
mod did;
pub mod models;
pub mod mst;
mod repo;
mod ucan_p256;
mod vendored;
mod web;

use bsky::*;
pub use crypto::{KeyPair, PubKey};
pub use db::AtpDatabase;
pub use did::DidDocMeta;
pub use models::*;
pub use repo::{Mutation, RepoCommit, RepoStore};
pub use ucan_p256::P256KeyMaterial;
use web::*;

pub struct AtpService {
    pub repo: RepoStore,
    pub atp_db: AtpDatabase,
    pub pds_keypair: KeyPair,
    pub tid_gen: Ticker,
    pub config: AtpServiceConfig,
}

#[derive(Clone, Debug)]
pub struct AtpServiceConfig {
    pub listen_host_port: String,
    pub public_url: String,
    pub registration_domain: Option<String>,
    pub invite_code: Option<String>,
    pub homepage_handle: Option<String>,
}

impl Default for AtpServiceConfig {
    fn default() -> Self {
        AtpServiceConfig {
            listen_host_port: "localhost:3030".to_string(),
            public_url: "http://localhost".to_string(),
            registration_domain: None,
            invite_code: None,
            homepage_handle: None,
        }
    }
}

#[derive(Debug)]
enum XrpcError {
    BadRequest(String),
    NotFound(String),
    Forbidden(String),
    MutexPoisoned,
}

impl std::error::Error for XrpcError {}

impl fmt::Display for XrpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadRequest(msg) | Self::NotFound(msg) | Self::Forbidden(msg) => {
                write!(f, "{}", msg)
            }
            Self::MutexPoisoned => write!(f, "service mutex poisoned"),
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
                // crash hard on mutex poison error
                Some(XrpcError::MutexPoisoned) => std::process::exit(-1),
                None => 500,
            };
            warn!("HTTP {}: {}", code, msg);
            Response::json(&json!({ "message": msg })).with_status_code(code)
        }
    }
}

/// Helper to take a Askama render Result and transform it to a rouille response, including more
/// friendly HTML 404s, etc.
fn web_wrap(resp: Result<String>) -> Response {
    match resp {
        Ok(val) => Response::html(val),
        Err(e) => {
            let msg = e.to_string();
            let code = match e.downcast_ref::<XrpcError>() {
                Some(XrpcError::BadRequest(_)) => 400,
                Some(XrpcError::NotFound(_)) => 404,
                Some(XrpcError::Forbidden(_)) => 403,
                // crash hard on mutex poison error
                Some(XrpcError::MutexPoisoned) => std::process::exit(-1),
                None => 500,
            };
            warn!("HTTP {}: {}", code, msg);
            let view = ErrorView {
                domain: "ERROR".to_string(),
                status_code: code,
                error_message: msg,
            };
            Response::html(view.render().unwrap())
        }
    }
}

impl AtpService {
    pub fn new(
        blockstore_db_path: &PathBuf,
        atp_db_path: &PathBuf,
        keypair: KeyPair,
        config: AtpServiceConfig,
    ) -> Result<Self> {
        Ok(AtpService {
            repo: RepoStore::open(blockstore_db_path)?,
            atp_db: AtpDatabase::open(atp_db_path)?,
            pds_keypair: keypair,
            tid_gen: Ticker::new(),
            config,
        })
    }

    pub fn new_ephemeral() -> Result<Self> {
        Ok(AtpService {
            repo: RepoStore::open_ephemeral()?,
            atp_db: AtpDatabase::open_ephemeral()?,
            pds_keypair: KeyPair::new_random(),
            tid_gen: Ticker::new(),
            config: AtpServiceConfig::default(),
        })
    }

    pub fn run_server(self) -> Result<()> {
        let config = self.config.clone();
        let srv = Mutex::new(self);

        let log_ok = |req: &Request, resp: &Response, elap: std::time::Duration| {
            info!(
                "{} {} ({}, {:?})",
                req.method(),
                req.raw_url(),
                resp.status_code,
                elap
            );
        };
        let log_err = |req: &Request, elap: std::time::Duration| {
            error!(
                "HTTP handler panicked: {} {} ({:?})",
                req.method(),
                req.raw_url(),
                elap
            );
        };

        rouille::start_server(config.listen_host_port, move |request| {
            rouille::log_custom(request, log_ok, log_err, || {
                router!(request,
                    // ============= Web Interface
                    (GET) ["/"] => {
                        if let Some(ref handle) = config.homepage_handle {
                            web_wrap(account_view_handler(&srv, handle, request))
                        } else {
                            web_wrap(home_view_handler(&srv, request))
                        }
                    },
                    (GET) ["/.well-known/did.json"] => {
                        match did_doc_view_handler(&srv, request) {
                            Ok(resp) => resp,
                            Err(e) => web_wrap(Err(e)),
                        }
                    },
                    (GET) ["/about"] => {
                        let host = request.header("Host").unwrap_or("localhost");
                        let view = AboutView { domain: host.to_string() };
                        Response::html(view.render().unwrap())
                    },
                    (GET) ["/u/{handle}", handle: String] => {
                        web_wrap(account_view_handler(&srv, &handle, request))
                    },
                    (GET) ["/u/{handle}/post/{tid}", handle: String, tid: Tid] => {
                        web_wrap(thread_view_handler(&srv, &handle, &tid, request))
                    },
                    (GET) ["/at/{did}", did: Did] => {
                        web_wrap(repo_view_handler(&srv, &did, request))
                    },
                    (GET) ["/at/{did}/{collection}", did: Did, collection: Nsid] => {
                        web_wrap(collection_view_handler(&srv, &did, &collection, request))
                    },
                    (GET) ["/at/{did}/{collection}/{tid}", did: Did, collection: Nsid, tid: Tid] => {
                        web_wrap(record_view_handler(&srv, &did, &collection, &tid, request))
                    },
                    // ============ Static Files (compiled in to executable)
                    (GET) ["/static/adenosine.css"] => {
                        Response::from_data("text/css", include_str!("../templates/adenosine.css"))
                    },
                    (GET) ["/static/favicon.png"] => {
                        Response::from_data("image/png", include_bytes!("../templates/favicon.png").to_vec())
                    },
                    (GET) ["/static/logo_128.png"] => {
                        Response::from_data("image/png", include_bytes!("../templates/logo_128.png").to_vec())
                    },
                    (GET) ["/robots.txt"] => {
                        Response::text(include_str!("../templates/robots.txt"))
                    },
                    // ============ XRPC AT Protocol
                    (POST) ["/xrpc/{endpoint}", endpoint: String] => {
                        xrpc_wrap(xrpc_post_handler(&srv, &endpoint, request))
                    },
                    (GET) ["/xrpc/com.atproto.sync.getRepo"] => {
                        // this one endpoint returns CAR file, not JSON, so wrappers don't work
                        match xrpc_get_repo_handler(&srv, request) {
                            Ok(car_bytes) => Response::from_data("application/octet-stream", car_bytes),
                            Err(e) => {
                                let msg = e.to_string();
                                let code = match e.downcast_ref::<XrpcError>() {
                                    Some(XrpcError::BadRequest(_)) => 400,
                                    Some(XrpcError::NotFound(_)) => 404,
                                    Some(XrpcError::Forbidden(_)) => 403,
                                    // crash hard on mutex poison error
                                    Some(XrpcError::MutexPoisoned) => std::process::exit(-1),
                                    None => 500,
                                };
                                warn!("HTTP {}: {}", code, msg);
                                Response::json(&json!({ "message": msg })).with_status_code(code)
                            }
                        }
                    },
                    (GET) ["/xrpc/{endpoint}", endpoint: String] => {
                        xrpc_wrap(xrpc_get_handler(&srv, &endpoint, request))
                    },
                    _ => web_wrap(Err(XrpcError::NotFound("unknown URL pattern".to_string()).into())),
                )
            })
        });
    }
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
        "com.atproto.server.getAccountsConfig" => {
            let srv = srv.lock().or(Err(XrpcError::MutexPoisoned))?;
            let mut avail_domains = vec![];
            if let Some(domain) = &srv.config.registration_domain {
                avail_domains.push(domain)
            }
            // TODO: optional "links" object with "privacyPolicy" and "termsOfService" URLs
            Ok(
                json!({"availableUserDomains": avail_domains, "inviteCodeRequired": srv.config.invite_code.is_some()}),
            )
        }
        "com.atproto.repo.getRecord" => {
            let did = Did::from_str(&xrpc_required_param(request, "user")?)?;
            let collection = Nsid::from_str(&xrpc_required_param(request, "collection")?)?;
            let rkey = Tid::from_str(&xrpc_required_param(request, "rkey")?)?;
            let mut srv = srv.lock().or(Err(XrpcError::MutexPoisoned))?;
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
        "com.atproto.sync.getRoot" => {
            let did = Did::from_str(&xrpc_required_param(request, "did")?)?;
            let mut srv = srv.lock().or(Err(XrpcError::MutexPoisoned))?;
            srv.repo
                .lookup_commit(&did)?
                .map(|v| json!({ "root": v.to_string() }))
                .ok_or(XrpcError::NotFound(format!("no repository found for DID: {}", did)).into())
        }
        "com.atproto.repo.listRecords" => {
            // TODO: limit, before, after, tid, reverse
            // TODO: handle non-DID 'user'
            // TODO: limit result set size
            let did = Did::from_str(&xrpc_required_param(request, "user")?)?;
            let collection = Nsid::from_str(&xrpc_required_param(request, "collection")?)?;
            let mut record_list: Vec<Value> = vec![];
            let mut srv = srv.lock().or(Err(XrpcError::MutexPoisoned))?;
            let commit_cid = &srv.repo.lookup_commit(&did)?.unwrap();
            let last_commit = srv.repo.get_commit(commit_cid)?;
            let full_map = srv.repo.mst_to_map(&last_commit.mst_cid)?;
            let prefix = format!("/{}/", collection);
            for (mst_key, cid) in full_map.iter() {
                //debug!("{}", mst_key);
                if mst_key.starts_with(&prefix) {
                    let record = srv.repo.get_ipld(cid)?;
                    record_list.push(json!({
                        "uri": format!("at://{}{}", did, mst_key),
                        "cid": cid.to_string(),
                        "value": ipld_into_json_value(record),
                    }));
                }
            }
            Ok(json!({ "records": record_list }))
        }
        "com.atproto.session.get" => {
            let mut srv = srv.lock().or(Err(XrpcError::MutexPoisoned))?;
            let auth_did = &xrpc_check_auth_header(&mut srv, request, None)?;
            let handle = srv
                .atp_db
                .resolve_did(auth_did)?
                .expect("registered account has handle");
            Ok(json!({"did": auth_did.to_string(), "handle": handle}))
        }
        "com.atproto.handle.resolve" => {
            let handle = xrpc_required_param(request, "handle")?;
            let mut srv = srv.lock().or(Err(XrpcError::MutexPoisoned))?;
            match srv.atp_db.resolve_handle(&handle)? {
                Some(did) => Ok(json!({"did": did.to_string()})),
                None => Err(XrpcError::NotFound(format!(
                    "could not resolve handle internally: {}",
                    handle
                )))?,
            }
        }
        "com.atproto.repo.describe" => {
            let did = Did::from_str(&xrpc_required_param(request, "user")?)?;

            let mut srv = srv.lock().or(Err(XrpcError::MutexPoisoned))?;
            let did_doc = srv.atp_db.get_did_doc(&did)?;
            let collections: Vec<String> = srv.repo.collections(&did)?;
            let desc = RepoDescribe {
                name: did.to_string(), // TODO: handle?
                did: did.to_string(),
                didDoc: did_doc,
                collections,
                nameIsCorrect: true,
            };
            Ok(json!(desc))
        }
        // =========== app.bsky methods
        "app.bsky.actor.getProfile" => {
            // TODO did or handle
            let did = Did::from_str(&xrpc_required_param(request, "actor")?)?;
            let mut srv = srv.lock().unwrap();
            // TODO: if profile doesn't exist, return a 404
            Ok(json!(bsky_get_profile(&mut srv, &did)?))
        }
        "app.bsky.actor.search" => {
            // TODO: actual implementation
            let _term = xrpc_required_param(request, "term")?;
            Ok(json!({"users": []}))
        }
        "app.bsky.actor.searchTypeahead" => {
            // TODO: actual implementation
            let _term = xrpc_required_param(request, "term")?;
            Ok(json!({"users": []}))
        }
        "app.bsky.actor.getSuggestions" => {
            // TODO: actual implementation
            Ok(json!({"actors": []}))
        }
        "app.bsky.feed.getAuthorFeed" => {
            // TODO did or handle
            let did = Did::from_str(&xrpc_required_param(request, "author")?)?;
            let mut srv = srv.lock().unwrap();
            Ok(json!(bsky_get_author_feed(&mut srv, &did)?))
        }
        "app.bsky.feed.getTimeline" => {
            let mut srv = srv.lock().unwrap();
            let auth_did = &xrpc_check_auth_header(&mut srv, request, None)?;
            Ok(json!(bsky_get_timeline(&mut srv, auth_did)?))
        }
        "app.bsky.feed.getPostThread" => {
            let uri = AtUri::from_str(&xrpc_required_param(request, "uri")?)?;
            let mut srv = srv.lock().unwrap();
            Ok(json!(bsky_get_thread(&mut srv, &uri, None)?))
        }
        _ => Err(anyhow!(XrpcError::NotFound(format!(
            "XRPC endpoint handler not found: {}",
            method
        )))),
    }
}

fn xrpc_get_repo_handler(srv: &Mutex<AtpService>, request: &Request) -> Result<Vec<u8>> {
    let did = Did::from_str(&xrpc_required_param(request, "did")?)?;
    let mut srv = srv.lock().or(Err(XrpcError::MutexPoisoned))?;
    // TODO: don't unwrap here
    let commit_cid = srv.repo.lookup_commit(&did)?.unwrap();
    srv.repo.export_car(&commit_cid, None)
}

pub fn create_account(
    srv: &mut AtpService,
    req: &AccountRequest,
    create_did_plc: bool,
) -> Result<AtpSession> {
    // check if account already exists (fast path, also confirmed by database schema)
    if srv.atp_db.account_exists(&req.handle, &req.email)? {
        Err(XrpcError::BadRequest(
            "handle or email already exists".to_string(),
        ))?;
    };

    debug!("trying to create new account: {}", &req.handle);

    let (did, did_doc) = if create_did_plc {
        // generate DID
        let create_op = did::CreateOp::new(
            req.handle.clone(),
            srv.config.public_url.clone(),
            &srv.pds_keypair,
            req.recoveryKey.clone(),
        );
        create_op.verify_self()?;
        let did = create_op.did_plc();
        let did_doc = create_op.did_doc();
        (did, did_doc)
    } else {
        let did = Did::from_str(&format!("did:web:{}", req.handle))?;
        let signing_key = srv.pds_keypair.pubkey().to_did_key();
        let recovery_key = req.recoveryKey.clone().unwrap_or(signing_key.clone());
        let meta = DidDocMeta {
            did: did.clone(),
            user_url: format!("https://{}", req.handle),
            service_url: srv.config.public_url.clone(),
            recovery_didkey: recovery_key,
            signing_didkey: signing_key,
        };
        (did, meta.did_doc())
    };

    // register in ATP DB and generate DID doc
    let recovery_key = req
        .recoveryKey
        .clone()
        .unwrap_or(srv.pds_keypair.pubkey().to_did_key());
    srv.atp_db
        .create_account(&did, &req.handle, &req.password, &req.email, &recovery_key)?;
    srv.atp_db.put_did_doc(&did, &did_doc)?;

    // insert empty MST repository
    let root_cid = {
        let empty_map_cid = srv.repo.mst_from_map(&Default::default())?;
        let meta_cid = srv.repo.write_metadata(&did)?;
        srv.repo.write_root(meta_cid, None, empty_map_cid)?
    };
    let _commit_cid = srv.repo.write_commit(&did, root_cid, "XXX-dummy-sig")?;

    let keypair = srv.pds_keypair.clone();
    let sess = srv
        .atp_db
        .create_session(&req.handle, &req.password, &keypair)?;
    Ok(sess)
}

fn xrpc_post_handler(
    srv: &Mutex<AtpService>,
    method: &str,
    request: &Request,
) -> Result<serde_json::Value> {
    match method {
        "com.atproto.account.create" => {
            // validate account request
            let req: AccountRequest = rouille::input::json_input(request)
                .map_err(|e| XrpcError::BadRequest(format!("failed to parse JSON body: {}", e)))?;
            // TODO: validate handle, email, recoverykey
            let mut srv = srv.lock().unwrap();
            if let Some(ref domain) = srv.config.registration_domain {
                // TODO: better matching, should not allow arbitrary sub-domains
                if !req.handle.ends_with(domain) {
                    Err(XrpcError::BadRequest(format!(
                        "handle is not under registration domain ({})",
                        domain
                    )))?;
                }
            } else {
                Err(XrpcError::BadRequest(
                    "account registration is disabled on this PDS".to_string(),
                ))?;
            };
            if srv.config.invite_code.is_some() && srv.config.invite_code != req.inviteCode {
                Err(XrpcError::Forbidden(
                    "a valid invite code is required".to_string(),
                ))?;
            };
            let sess = create_account(&mut srv, &req, true)?;
            Ok(json!(sess))
        }
        "com.atproto.session.create" => {
            let req: SessionRequest = rouille::input::json_input(request)
                .map_err(|e| XrpcError::BadRequest(format!("failed to parse JSON body: {}", e)))?;
            let mut srv = srv.lock().unwrap();
            let keypair = srv.pds_keypair.clone();
            Ok(json!(srv.atp_db.create_session(
                &req.handle,
                &req.password,
                &keypair
            )?))
        }
        "com.atproto.session.refresh" => {
            // actually just returns current session, because we don't implement refresh
            let mut srv = srv.lock().unwrap();
            let did = xrpc_check_auth_header(&mut srv, request, None)?;
            let header = request
                .header("Authorization")
                .ok_or(XrpcError::Forbidden("require auth header".to_string()))?;
            if !header.starts_with("Bearer ") {
                Err(XrpcError::Forbidden("require bearer token".to_string()))?;
            }
            let jwt = header.split(' ').nth(1).unwrap();
            let handle = srv
                .atp_db
                .resolve_did(&did)?
                .expect("DID matches to a handle");

            Ok(json!(AtpSession {
                did: did.to_string(),
                name: handle,
                accessJwt: jwt.to_string(),
                refreshJwt: jwt.to_string(),
            }))
        }
        "com.atproto.session.delete" => {
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
        "com.atproto.repo.batchWrite" => {
            let batch: RepoBatchWriteBody = rouille::input::json_input(request)?;
            // TODO: validate edits against schemas
            let did = Did::from_str(&batch.did)?;
            let mut srv = srv.lock().unwrap();
            let _auth_did = &xrpc_check_auth_header(&mut srv, request, Some(&did))?;
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
            let keypair = srv.pds_keypair.clone();
            srv.repo.mutate_repo(&did, &mutations, &keypair)?;
            bsky_mutate_db(&mut srv.atp_db, &did, mutations)?;
            Ok(json!({}))
        }
        "com.atproto.repo.createRecord" => {
            // TODO: validate edits against schemas
            let create: RepoCreateRecord = rouille::input::json_input(request)?;
            let did = Did::from_str(&create.did)?;
            let collection = Nsid::from_str(&create.collection)?;
            let mut srv = srv.lock().unwrap();
            let _auth_did = &xrpc_check_auth_header(&mut srv, request, Some(&did))?;
            let mutations: Vec<Mutation> = vec![Mutation::Create(
                collection,
                srv.tid_gen.next_tid(),
                json_value_into_ipld(create.record),
            )];
            let keypair = srv.pds_keypair.clone();
            srv.repo.mutate_repo(&did, &mutations, &keypair)?;
            bsky_mutate_db(&mut srv.atp_db, &did, mutations)?;
            Ok(json!({}))
        }
        "com.atproto.repo.putRecord" => {
            // TODO: validate edits against schemas
            let put: RepoPutRecord = rouille::input::json_input(request)?;
            let did = Did::from_str(&put.did)?;
            let collection = Nsid::from_str(&put.collection)?;
            let tid = Tid::from_str(&put.rkey)?;
            let mut srv = srv.lock().unwrap();
            let _auth_did = &xrpc_check_auth_header(&mut srv, request, Some(&did))?;

            let mutations: Vec<Mutation> = vec![Mutation::Update(
                collection,
                tid,
                json_value_into_ipld(put.record),
            )];
            let keypair = srv.pds_keypair.clone();
            srv.repo.mutate_repo(&did, &mutations, &keypair)?;
            bsky_mutate_db(&mut srv.atp_db, &did, mutations)?;
            Ok(json!({}))
        }
        "com.atproto.repo.deleteRecord" => {
            let delete: RepoDeleteRecord = rouille::input::json_input(request)?;
            let did = Did::from_str(&delete.did)?;
            let collection = Nsid::from_str(&delete.collection)?;
            let tid = Tid::from_str(&delete.rkey)?;
            let mut srv = srv.lock().unwrap();
            let _auth_did = &xrpc_check_auth_header(&mut srv, request, Some(&did))?;

            let mutations: Vec<Mutation> = vec![Mutation::Delete(collection, tid)];
            let keypair = srv.pds_keypair.clone();
            srv.repo.mutate_repo(&did, &mutations, &keypair)?;
            bsky_mutate_db(&mut srv.atp_db, &did, mutations)?;
            Ok(json!({}))
        }
        "com.atproto.sync.updateRepo" => {
            // TODO: all other XRPC POST methods removed params (eg, 'did' in this case)
            let did = Did::from_str(&xrpc_required_param(request, "did")?)?;
            // important that this read is before we take the mutex, because it could be slow!
            let mut car_bytes: Vec<u8> = Default::default();
            // TODO: unwrap()
            request.data().unwrap().read_to_end(&mut car_bytes)?;
            let mut srv = srv.lock().unwrap();
            let _auth_did = &xrpc_check_auth_header(&mut srv, request, Some(&did))?;
            srv.repo
                .import_car_bytes(&car_bytes, Some(did.to_string()))?;
            // TODO: need to update atp_db
            Ok(json!({}))
        }
        // =========== app.bsky methods
        "app.bsky.actor.updateProfile" => {
            let profile: ProfileRecord = rouille::input::json_input(request)?;
            let mut srv = srv.lock().unwrap();
            let auth_did = &xrpc_check_auth_header(&mut srv, request, None)?;
            bsky_update_profile(&mut srv, auth_did, profile)?;
            Ok(json!({}))
        }
        _ => Err(anyhow!(XrpcError::NotFound(format!(
            "XRPC endpoint handler not found: {}",
            method
        )))),
    }
}

fn home_view_handler(srv: &Mutex<AtpService>, request: &Request) -> Result<String> {
    let host = request.header("Host").unwrap_or("localhost");

    // check if the hostname resolves to a DID (account)
    let did: Option<Did> = {
        // this mutex lock should drop at the end of this block
        let mut srv = srv.lock().or(Err(XrpcError::MutexPoisoned))?;
        srv.atp_db.resolve_handle(host)?
    };
    if did.is_some() {
        account_view_handler(srv, host, request)
    } else {
        let view = GenericHomeView {
            domain: host.to_string(),
        };
        Ok(view.render()?)
    }
}

fn did_doc_view_handler(srv: &Mutex<AtpService>, request: &Request) -> Result<Response> {
    let host = request.header("Host").unwrap_or("localhost");
    let mut srv = srv.lock().or(Err(XrpcError::MutexPoisoned))?;
    if let Some(did) = srv.atp_db.resolve_handle(host)? {
        if did.to_string().starts_with("did:web:") {
            let did_doc = srv.atp_db.get_did_doc(&did)?;
            return Ok(Response::json(&did_doc));
        }
    };
    Err(XrpcError::NotFound(
        "no did:web: account registered at this domain".to_string(),
    ))?
}

// TODO: did, collection, tid have already been parsed by this point
fn account_view_handler(
    srv: &Mutex<AtpService>,
    handle: &str,
    request: &Request,
) -> Result<String> {
    let host = request.header("Host").unwrap_or("localhost");
    let mut srv = srv.lock().or(Err(XrpcError::MutexPoisoned))?;
    // TODO: unwrap as 404
    let did = srv
        .atp_db
        .resolve_handle(handle)?
        .ok_or(XrpcError::NotFound(format!(
            "no DID found for handle: {}",
            handle
        )))?;

    Ok(AccountView {
        domain: host.to_string(),
        did: did.clone(),
        profile: bsky_get_profile(&mut srv, &did)?,
        feed: bsky_get_author_feed(&mut srv, &did)?.feed,
    }
    .render()?)
}

fn thread_view_handler(
    srv: &Mutex<AtpService>,
    handle: &str,
    tid: &Tid,
    request: &Request,
) -> Result<String> {
    let host = request.header("Host").unwrap_or("localhost");
    let collection = Nsid::from_str("app.bsky.feed.post")?;
    let mut srv = srv.lock().or(Err(XrpcError::MutexPoisoned))?;
    // TODO: not unwrap
    let did = srv.atp_db.resolve_handle(handle)?.unwrap();

    // TODO: could construct URI directly
    let uri = AtUri::from_str(&format!("at://{}/{}/{}", did, collection, tid))?;
    Ok(ThreadView {
        domain: host.to_string(),
        did,
        collection,
        tid: tid.clone(),
        post: bsky_get_thread(&mut srv, &uri, None)?.thread,
    }
    .render()?)
}

fn repo_view_handler(srv: &Mutex<AtpService>, did: &str, request: &Request) -> Result<String> {
    let host = request.header("Host").unwrap_or("localhost");
    let did = Did::from_str(did)?;

    let mut srv = srv.lock().or(Err(XrpcError::MutexPoisoned))?;
    let did_doc = srv.atp_db.get_did_doc(&did)?;
    let commit_cid = &srv.repo.lookup_commit(&did)?.unwrap();
    let commit = srv.repo.get_commit(commit_cid)?;
    let collections: Vec<String> = srv.repo.collections(&did)?;
    let desc = RepoDescribe {
        name: did.to_string(), // TODO
        did: did.to_string(),
        didDoc: did_doc,
        collections,
        nameIsCorrect: true,
    };

    Ok(RepoView {
        domain: host.to_string(),
        did,
        commit,
        describe: desc,
    }
    .render()?)
}

fn collection_view_handler(
    srv: &Mutex<AtpService>,
    did: &str,
    collection: &str,
    request: &Request,
) -> Result<String> {
    let host = request.header("Host").unwrap_or("localhost");
    let did = Did::from_str(did)?;
    let collection = Nsid::from_str(collection)?;

    let mut record_list: Vec<Value> = vec![];
    let mut srv = srv.lock().or(Err(XrpcError::MutexPoisoned))?;
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
                "tid": mst_key.split('/').nth(2).unwrap(),
                "cid": cid,
                "value": ipld_into_json_value(record),
            }));
        }
    }

    Ok(CollectionView {
        domain: host.to_string(),
        did,
        collection,
        records: record_list,
    }
    .render()?)
}

fn record_view_handler(
    srv: &Mutex<AtpService>,
    did: &str,
    collection: &str,
    tid: &str,
    request: &Request,
) -> Result<String> {
    let host = request.header("Host").unwrap_or("localhost");
    let did = Did::from_str(did)?;
    let collection = Nsid::from_str(collection)?;
    let rkey = Tid::from_str(tid)?;

    let mut srv = srv.lock().or(Err(XrpcError::MutexPoisoned))?;
    let key = format!("/{}/{}", collection, rkey);
    let record = match srv.repo.get_atp_record(&did, &collection, &rkey) {
        Ok(Some(ipld)) => ipld_into_json_value(ipld),
        Ok(None) => Err(anyhow!(XrpcError::NotFound(format!(
            "could not find record: {}",
            key
        ))))?,
        Err(e) => Err(e)?,
    };
    Ok(RecordView {
        domain: host.to_string(),
        did,
        collection,
        tid: rkey,
        record,
    }
    .render()?)
}
