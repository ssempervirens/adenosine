/// Helper program to import an IPLD CARv1 file in to sqlite data store
use anyhow::{anyhow, Result};
use futures::TryStreamExt;
use ipfs_sqlite_block_store::BlockStore;
use iroh_car::CarReader;
use libipld::block::Block;
use tokio::fs::File;
use tokio::io::BufReader;

use std::env;

async fn load_car_to_sqlite(db_path: &str, car_path: &str) -> Result<()> {
    let car_reader = {
        let file = File::open(car_path).await?;
        let buf_reader = BufReader::new(file);
        CarReader::new(buf_reader).await?
    };
    let car_header = car_reader.header().clone();
    let mut db: BlockStore<libipld::DefaultParams> = {
        let path = std::path::PathBuf::from(db_path);
        let path = ipfs_sqlite_block_store::DbPath::File(path);
        BlockStore::open_path(path, Default::default())?
    };

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

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("expected 2 args: <db_path> <car_path>");
        std::process::exit(-1);
    }
    let db_path = &args[1];
    let car_path = &args[2];
    load_car_to_sqlite(db_path, car_path).await
}
