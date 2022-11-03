use anyhow::Result;

use futures::TryStreamExt;
use ipfs_sqlite_block_store::BlockStore;
use iroh_car::CarReader;
use libipld::{Block, Cid};
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::BufReader;

pub fn load_car_to_sqlite(db_path: &PathBuf, car_path: &PathBuf) -> Result<()> {
    let mut db: BlockStore<libipld::DefaultParams> =
        { BlockStore::open(db_path, Default::default())? };

    load_car_to_blockstore(&mut db, car_path)?;
    Ok(())
}

pub fn load_car_to_blockstore(
    db: &mut BlockStore<libipld::DefaultParams>,
    car_path: &PathBuf,
) -> Result<Cid> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(inner_car_loader(db, car_path))
}

// this async function is wrapped in the sync version above
async fn inner_car_loader(
    db: &mut BlockStore<libipld::DefaultParams>,
    car_path: &PathBuf,
) -> Result<Cid> {
    println!(
        "{} - {}",
        std::env::current_dir()?.display(),
        car_path.display()
    );
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

    // pin the header (?)
    if car_header.roots().len() >= 1 {
        db.alias(b"import".to_vec(), Some(&car_header.roots()[0]))?;
    }

    Ok(car_header.roots()[0])
}
