use anyhow::Result;

use crate::vendored::iroh_car::{CarHeader, CarReader, CarWriter};
use futures::TryStreamExt;
use ipfs_sqlite_block_store::BlockStore;
use libipld::{Block, Cid};
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncRead, BufReader};

/// Synchronous wrapper for loading in-memory CAR bytes (`&[u8]`) into a blockstore.
///
/// Does not do any pinning, even temporarily. Returns the root CID indicated in the CAR file
/// header.
pub fn load_car_bytes_to_blockstore(
    db: &mut BlockStore<libipld::DefaultParams>,
    car_bytes: &[u8],
) -> Result<Cid> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(inner_car_bytes_loader(db, car_bytes))
}

/// Synchronous wrapper for loading on-disk CAR file (by path) into a blockstore.
///
/// Does not do any pinning, even temporarily. Returns the root CID indicated in the CAR file
/// header.
pub fn load_car_path_to_blockstore(
    db: &mut BlockStore<libipld::DefaultParams>,
    car_path: &PathBuf,
) -> Result<Cid> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(inner_car_path_loader(db, car_path))
}

pub fn read_car_bytes_from_blockstore(
    db: &mut BlockStore<libipld::DefaultParams>,
    root: &Cid,
) -> Result<Vec<u8>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(inner_car_bytes_reader(db, root))
}

async fn inner_car_bytes_loader(
    db: &mut BlockStore<libipld::DefaultParams>,
    car_bytes: &[u8],
) -> Result<Cid> {
    let car_reader = CarReader::new(car_bytes).await?;
    inner_car_loader(db, car_reader).await
}

async fn inner_car_path_loader(
    db: &mut BlockStore<libipld::DefaultParams>,
    car_path: &PathBuf,
) -> Result<Cid> {
    let car_reader = {
        let file = File::open(car_path).await?;
        let buf_reader = BufReader::new(file);
        CarReader::new(buf_reader).await?
    };
    inner_car_loader(db, car_reader).await
}

async fn inner_car_loader<R: AsyncRead + Send + Unpin>(
    db: &mut BlockStore<libipld::DefaultParams>,
    car_reader: CarReader<R>,
) -> Result<Cid> {
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
    Ok(car_header.roots()[0])
}

async fn inner_car_bytes_reader(
    db: &mut BlockStore<libipld::DefaultParams>,
    root: &Cid,
) -> Result<Vec<u8>> {
    let car_header = CarHeader::new_v1(vec![root.clone()]);
    let buf: Vec<u8> = Default::default();
    let mut car_writer = CarWriter::new(car_header, buf);

    let cid_list = db.get_descendants::<Vec<_>>(root)?;
    for cid in cid_list {
        let block = db.get_block(&cid)?.expect("block content");
        car_writer.write(cid, block).await?;
    }
    Ok(car_writer.finish().await?)
}
