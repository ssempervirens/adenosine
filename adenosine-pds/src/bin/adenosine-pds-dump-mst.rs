/// Helper program to print MST keys/docs from a sqlite repo
use anyhow::{anyhow, Result};
use ipfs_sqlite_block_store::BlockStore;
use libipld::cbor::DagCborCodec;
use libipld::prelude::Codec;
use libipld::{Cid, DagCbor};

use std::env;

#[derive(Debug, DagCbor, PartialEq, Eq)]
struct CommitNode {
    root: Cid,
    sig: Box<[u8]>,
}

#[derive(Debug, DagCbor, PartialEq, Eq)]
struct RootNode {
    auth_token: Option<String>,
    prev: Option<Cid>,
    // TODO: not 'metadata'?
    meta: Cid,
    data: Cid,
}

#[derive(Debug, DagCbor, PartialEq, Eq)]
struct MetadataNode {
    datastore: String, // "mst"
    did: String,
    version: u8, // 1
}

#[derive(Debug, DagCbor, PartialEq, Eq)]
struct MstEntry {
    k: String,
    p: u32,
    v: Cid,
    t: Option<Cid>,
}

#[derive(Debug, DagCbor, PartialEq)]
struct MstNode {
    l: Option<Cid>,
    e: Vec<MstEntry>,
}

fn get_mst_node(db: &mut BlockStore<libipld::DefaultParams>, cid: &Cid) -> Result<MstNode> {
    let mst_node: MstNode = DagCborCodec.decode(
        &db.get_block(cid)?
            .ok_or(anyhow!("expected block in store"))?,
    )?;
    Ok(mst_node)
}

fn print_mst_keys(db: &mut BlockStore<libipld::DefaultParams>, cid: &Cid) -> Result<()> {
    let node = get_mst_node(db, cid)?;
    if let Some(ref left) = node.l {
        print_mst_keys(db, left)?;
    }
    let mut key: String = "".to_string();
    for entry in node.e.iter() {
        key = format!("{}{}", &key[0..entry.p as usize], entry.k);
        println!("{}\t-> {}", key, entry.v);
        if let Some(ref right) = entry.t {
            print_mst_keys(db, right)?;
        }
    }
    Ok(())
}

async fn dump_mst_keys(db_path: &str) -> Result<()> {
    let mut db: BlockStore<libipld::DefaultParams> = {
        let path = std::path::PathBuf::from(db_path);
        let path = ipfs_sqlite_block_store::DbPath::File(path);
        BlockStore::open_path(path, Default::default())?
    };

    let all_aliases: Vec<(Vec<u8>, Cid)> = db.aliases()?;
    if all_aliases.is_empty() {
        println!("expected at least one alias in block store");
        std::process::exit(-1);
    }
    let (alias, commit_cid) = all_aliases[0].clone();
    println!(
        "starting from {} [{}]",
        commit_cid,
        String::from_utf8_lossy(&alias)
    );

    // NOTE: the faster way to develop would have been to decode to libipld::ipld::Ipld first? meh

    //println!("raw commit: {:?}", &db.get_block(&commit_cid)?.ok_or(anyhow!("expected commit block in store"))?);
    let commit: CommitNode = DagCborCodec.decode(
        &db.get_block(&commit_cid)?
            .ok_or(anyhow!("expected commit block in store"))?,
    )?;
    println!("Commit: {:?}", commit);
    //println!("raw root: {:?}", &db.get_block(&commit.root)?.ok_or(anyhow!("expected commit block in store"))?);
    let root: RootNode = DagCborCodec.decode(
        &db.get_block(&commit.root)?
            .ok_or(anyhow!("expected root block in store"))?,
    )?;
    println!("Root: {:?}", root);
    let metadata: MetadataNode = DagCborCodec.decode(
        &db.get_block(&root.meta)?
            .ok_or(anyhow!("expected metadata block in store"))?,
    )?;
    println!("Metadata: {:?}", metadata);
    let mst_node: MstNode = DagCborCodec.decode(
        &db.get_block(&root.data)?
            .ok_or(anyhow!("expected block in store"))?,
    )?;
    println!("MST root node: {:?}", mst_node);

    println!("============");

    print_mst_keys(&mut db, &root.data)?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("expected 1 args: <db_path>");
        std::process::exit(-1);
    }
    let db_path = &args[1];
    dump_mst_keys(db_path).await
}
