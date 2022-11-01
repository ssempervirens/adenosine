use anyhow::Result;
use log::{error, info};
use rouille::{router, Request, Response};
use std::path::PathBuf;
use std::sync::Mutex;

use ipfs_sqlite_block_store::BlockStore;

mod car;
mod db;
mod models;
mod mst;

pub use car::{load_car_to_blockstore, load_car_to_sqlite};
pub use db::AtpDatabase;
pub use models::*;
pub use mst::{dump_mst_keys, repro_mst};

pub fn run_server(port: u16, blockstore_db_path: &PathBuf, atp_db_path: &PathBuf) -> Result<()> {
    // TODO: some static files? https://github.com/tomaka/rouille/blob/master/examples/static-files.rs

    // TODO: could just open connection on every request?
    let db = Mutex::new(AtpDatabase::open(atp_db_path)?);
    let mut _blockstore: BlockStore<libipld::DefaultParams> =
        BlockStore::open(blockstore_db_path, Default::default())?;

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
                (GET) ["/xrpc/some.method"] => {
                    Response::text("didn't get a thing")
                    // TODO: reply with query params as a JSON body
                },
                (POST) ["/xrpc/other.method"] => {
                    Response::text("didn't get other thing")
                    // TODO: parse and echo back JSON body
                },

                (GET) ["/xrpc/com.atproto.getRecord"] => {
                    // TODO: JSON response
                    // TODO: handle error
                    let mut db = db.lock().unwrap().new_connection().unwrap();
                    Response::text(db.get_record("asdf", "123", "blah").unwrap().to_string())
                },
                _ => rouille::Response::empty_404()
            )
        })
    });
}
