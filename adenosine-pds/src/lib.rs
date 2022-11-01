use anyhow::Result;
use log::{error, info};
use rouille::{router, Request, Response};

use futures::TryStreamExt;
use ipfs_sqlite_block_store::BlockStore;
use iroh_car::CarReader;
use libipld::block::Block;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::BufReader;

mod mst;

pub use mst::{dump_mst_keys, repro_mst};

pub fn run_server(port: u16) -> Result<()> {
    // TODO: log access requests
    // TODO: some static files? https://github.com/tomaka/rouille/blob/master/examples/static-files.rs

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
                _ => rouille::Response::empty_404()
            )
        })
    });
}

pub fn load_car_to_sqlite(db_path: &PathBuf, car_path: &PathBuf) -> Result<()> {
    let mut db: BlockStore<libipld::DefaultParams> =
        { BlockStore::open(db_path, Default::default())? };

    load_car_to_blockstore(&mut db, car_path)
}

pub fn load_car_to_blockstore(db: &mut BlockStore<libipld::DefaultParams>, car_path: &PathBuf) -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(inner_car_loader(db, car_path))
}

// this async function is wrapped in the sync version above
async fn inner_car_loader(db: &mut BlockStore<libipld::DefaultParams>, car_path: &PathBuf) -> Result<()> {
    println!("{} - {}", std::env::current_dir()?.display(), car_path.display());
    let car_reader = {
        let file = File::open(car_path).await?;
        let buf_reader = BufReader::new(file);
        CarReader::new(buf_reader).await?
    };
    let car_header = car_reader.header().clone();

    car_reader
        .stream()
        .try_for_each(|(cid, raw)| {
            // TODO: error handling here instead of unwrap (?)
            let block = Block::new(cid, raw).unwrap();
            db.put_block(block, None).unwrap();
            futures::future::ready(Ok(()))
        })
        .await?;

    // pin the header
    if car_header.roots().len() >= 1 {
        db.alias(b"import".to_vec(), Some(&car_header.roots()[0]))?;
    }

    Ok(())
}
