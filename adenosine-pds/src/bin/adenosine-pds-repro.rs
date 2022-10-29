/// Development helper which loads MST keys and CIDs, re-generates MST structure, then compares
/// root node with what was originally found.
use anyhow::{anyhow, Result};
use ipfs_sqlite_block_store::BlockStore;
use libipld::cbor::DagCborCodec;
use libipld::multihash::Code;
use libipld::prelude::Codec;
use libipld::store::DefaultParams;
use libipld::Block;
use libipld::{Cid, DagCbor};
use std::collections::BTreeMap;

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

struct WipEntry {
    height: u8,
    key: String,
    val: Cid,
    right: Option<Box<WipNode>>,
}

struct WipNode {
    height: u8,
    left: Option<Box<WipNode>>,
    entries: Vec<WipEntry>,
}

fn get_mst_node(db: &mut BlockStore<libipld::DefaultParams>, cid: &Cid) -> Result<MstNode> {
    let mst_node: MstNode = DagCborCodec.decode(
        &db.get_block(cid)?
            .ok_or(anyhow!("expected block in store"))?,
    )?;
    Ok(mst_node)
}

fn collect_mst_keys(
    db: &mut BlockStore<libipld::DefaultParams>,
    cid: &Cid,
    map: &mut BTreeMap<String, Cid>,
) -> Result<()> {
    let node = get_mst_node(db, cid)?;
    if let Some(ref left) = node.l {
        collect_mst_keys(db, left, map)?;
    }
    let mut key: String = "".to_string();
    for entry in node.e.iter() {
        key = format!("{}{}", &key[0..entry.p as usize], entry.k);
        map.insert(key.clone(), entry.v);
        if let Some(ref right) = entry.t {
            collect_mst_keys(db, right, map)?;
        }
    }
    Ok(())
}

fn leading_zeros(key: &str) -> u8 {
    let digest = sha256::digest(key);
    let digest = digest.as_bytes();
    for i in 0..digest.len() {
        if digest[i] != '0' as u8 {
            return i as u8;
        }
    }
    digest.len() as u8
}

fn generate_mst(
    db: &mut BlockStore<libipld::DefaultParams>,
    map: &mut BTreeMap<String, Cid>,
) -> Result<Cid> {
    // construct a "WIP" tree
    let mut root: Option<WipNode> = None;
    for (key, val) in map {
        let height = leading_zeros(key);
        let entry = WipEntry {
            height,
            key: key.clone(),
            val: val.clone(),
            right: None,
        };
        if let Some(node) = root {
            root = Some(insert_entry(node, entry));
        } else {
            root = Some(WipNode {
                height: entry.height,
                left: None,
                entries: vec![entry],
            });
        }
    }
    serialize_wip_tree(db, root.expect("non-empty MST tree"))
}

fn insert_entry(mut node: WipNode, entry: WipEntry) -> WipNode {
    // if we are higher on tree than existing node, replace it with new layer first
    if entry.height > node.height {
        node = WipNode {
            height: entry.height,
            left: Some(Box::new(node)),
            entries: vec![],
        }
    };
    // if we are lower on tree, then need to descend first
    if entry.height < node.height {
        // we should never be lower down the left than an existing node, and always to the right
        let mut last = node.entries.pop().expect("hit empty existing entry list");
        assert!(entry.key > last.key);
        if last.right.is_some() {
            last.right = Some(Box::new(insert_entry(*last.right.unwrap(), entry)));
        } else {
            last.right = Some(Box::new(WipNode {
                height: entry.height,
                left: None,
                entries: vec![entry],
            }));
        }
        node.entries.push(last);
        return node;
    }
    // same height, simply append to end (but verify first)
    assert!(node.height == entry.height);
    if !node.entries.is_empty() {
        let last = &node.entries.last().unwrap();
        assert!(entry.key > last.key);
    }
    node.entries.push(entry);
    node
}

/// returns the length of common characters between the two strings. Strings must be simple ASCII,
/// which should hold for current ATP MST keys (collection plus TID)
fn common_prefix_len(a: &str, b: &str) -> usize {
    let a = a.as_bytes();
    let b = b.as_bytes();
    for i in 0..std::cmp::min(a.len(), b.len()) {
        if a[i] != b[i] {
            return i;
        }
    }
    // strings are the same, up to common length
    a.len()
}

#[test]
fn test_common_prefix_len() {
    assert_eq!(common_prefix_len("abc", "abc"), 3);
    assert_eq!(common_prefix_len("abcde", "abc"), 3);
    assert_eq!(common_prefix_len("abcde", "abb"), 2);
    assert_eq!(common_prefix_len("", "asdf"), 0);
}

fn serialize_wip_tree(
    db: &mut BlockStore<libipld::DefaultParams>,
    wip_node: WipNode,
) -> Result<Cid> {
    let left: Option<Cid> = if let Some(left) = wip_node.left {
        Some(serialize_wip_tree(db, *left)?)
    } else {
        None
    };

    let mut entries: Vec<MstEntry> = vec![];
    let mut last_key = "".to_string();
    for wip_entry in wip_node.entries {
        let right: Option<Cid> = if let Some(right) = wip_entry.right {
            Some(serialize_wip_tree(db, *right)?)
        } else {
            None
        };
        let prefix_len = common_prefix_len(&last_key, &wip_entry.key);
        entries.push(MstEntry {
            k: wip_entry.key[prefix_len..].to_string(),
            p: prefix_len as u32,
            v: wip_entry.val,
            t: right,
        });
        last_key = wip_entry.key;
    }
    let mst_node = MstNode {
        l: left,
        e: entries,
    };
    let block = Block::<DefaultParams>::encode(DagCborCodec, Code::Sha2_256, &mst_node)?;
    let cid = block.cid().clone();
    db.put_block(block, None)?;
    Ok(cid)
}

async fn repro_mst(db_path: &str) -> Result<()> {
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
    let (_alias, commit_cid) = all_aliases[0].clone();

    let commit_node: CommitNode = DagCborCodec.decode(
        &db.get_block(&commit_cid)?
            .ok_or(anyhow!("expected commit block in store"))?,
    )?;
    let root_node: RootNode = DagCborCodec.decode(
        &db.get_block(&commit_node.root)?
            .ok_or(anyhow!("expected root block in store"))?,
    )?;
    let _metadata_node: MetadataNode = DagCborCodec.decode(
        &db.get_block(&root_node.meta)?
            .ok_or(anyhow!("expected metadata block in store"))?,
    )?;

    // collect key/value sorted map of string/cid, as BTree
    let mut repo_map: BTreeMap<String, Cid> = BTreeMap::new();
    collect_mst_keys(&mut db, &root_node.data, &mut repo_map)?;

    for (k, v) in repo_map.iter() {
        println!("{}\t-> {}", k, v);
    }

    // now re-generate nodes
    let updated = generate_mst(&mut db, &mut repo_map)?;

    println!("original root: {}", root_node.data);
    println!("regenerated  : {}", updated);
    if root_node.data == updated {
        println!("SUCCESS! (amazing)");
    } else {
        println!("FAILED");
        let a = get_mst_node(&mut db, &root_node.data)?;
        let b = get_mst_node(&mut db, &updated)?;
        println!("A: {:?}", a);
        println!("B: {:?}", b);
    };
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
    repro_mst(db_path).await
}
