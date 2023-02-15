use anyhow::{anyhow, Context, Result};
use ipfs_sqlite_block_store::BlockStore;
use libipld::cbor::DagCborCodec;
use libipld::multihash::Code;
use libipld::prelude::Codec;
use libipld::store::DefaultParams;
use libipld::Block;
use libipld::{Cid, DagCbor};
use log::{debug, error, info};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, DagCbor, PartialEq, Eq)]
pub struct CommitNode {
    pub root: Cid,
    pub sig: Box<[u8]>,
}

#[derive(Debug, DagCbor, PartialEq, Eq)]
pub struct RootNode {
    pub auth_token: Option<String>,
    pub prev: Option<Cid>,
    // TODO: not 'metadata'?
    pub meta: Cid,
    pub data: Cid,
}

#[derive(Debug, DagCbor, PartialEq, Eq)]
pub struct MetadataNode {
    pub datastore: String, // "mst"
    pub did: String,
    pub version: u8, // 1
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
    let block = &db
        .get_block(cid)?
        .ok_or(anyhow!("reading MST node from blockstore"))?;
    //println!("{:?}", block);
    let mst_node: MstNode = DagCborCodec
        .decode(block)
        .context("parsing MST DAG-CBOR IPLD node from blockstore")?;
    Ok(mst_node)
}

pub fn print_mst_keys(db: &mut BlockStore<libipld::DefaultParams>, cid: &Cid) -> Result<()> {
    let node = get_mst_node(db, cid)?;
    if let Some(ref left) = node.l {
        print_mst_keys(db, left)?;
    }
    let mut key: String = "".to_string();
    for entry in node.e.iter() {
        key = format!("{}{}", &key[0..entry.p as usize], entry.k);
        println!("\t{}\t-> {}", key, entry.v);
        if let Some(ref right) = entry.t {
            print_mst_keys(db, right)?;
        }
    }
    Ok(())
}

pub fn dump_mst_keys(db_path: &PathBuf) -> Result<()> {
    let mut db: BlockStore<libipld::DefaultParams> = BlockStore::open(db_path, Default::default())?;

    let all_aliases: Vec<(Vec<u8>, Cid)> = db.aliases()?;
    if all_aliases.is_empty() {
        error!("expected at least one alias in block store");
        std::process::exit(-1);
    }

    // print all the aliases
    for (alias, commit_cid) in all_aliases.iter() {
        let did = String::from_utf8_lossy(alias);
        println!("{} -> {}", did, commit_cid);
    }

    let (did, commit_cid) = all_aliases[0].clone();
    let did = String::from_utf8_lossy(&did);
    info!("starting from {} [{}]", commit_cid, did);

    // NOTE: the faster way to develop would have been to decode to libipld::ipld::Ipld first? meh

    debug!(
        "raw commit: {:?}",
        &db.get_block(&commit_cid)?
            .ok_or(anyhow!("expected commit block in store"))?
    );
    let commit: CommitNode = DagCborCodec.decode(
        &db.get_block(&commit_cid)?
            .ok_or(anyhow!("expected commit block in store"))?,
    )?;
    debug!("Commit: {:?}", commit);
    let root: RootNode = DagCborCodec.decode(
        &db.get_block(&commit.root)?
            .ok_or(anyhow!("expected root block in store"))?,
    )?;
    debug!("Root: {:?}", root);
    let metadata: MetadataNode = DagCborCodec.decode(
        &db.get_block(&root.meta)?
            .ok_or(anyhow!("expected metadata block in store"))?,
    )?;
    debug!("Metadata: {:?}", metadata);
    let mst_node: MstNode = DagCborCodec.decode(
        &db.get_block(&root.data)?
            .ok_or(anyhow!("expected block in store"))?,
    )?;
    debug!("MST root node: {:?}", mst_node);
    debug!("============");

    println!("{}", did);
    print_mst_keys(&mut db, &root.data)?;
    Ok(())
}

pub fn collect_mst_keys(
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
    for (i, c) in digest.iter().enumerate() {
        if *c != b'0' {
            return i as u8;
        }
    }
    digest.len() as u8
}

// # python code to generate test cases
// import hashlib
// seed = b"asdf"
// while True:
//     out = hashlib.sha256(seed).hexdigest()
//     if out.startswith("00"):
//         print(f"{seed} -> {out}")
//     seed = b"app.bsky.feed.post/" + out.encode('utf8')[:12]

#[test]
fn test_leading_zeros() {
    assert_eq!(leading_zeros(""), 0);
    assert_eq!(leading_zeros("asdf"), 0);
    assert_eq!(leading_zeros("2653ae71"), 0);
    assert_eq!(leading_zeros("88bfafc7"), 1);
    assert_eq!(leading_zeros("2a92d355"), 2);
    assert_eq!(leading_zeros("884976f5"), 3);
    assert_eq!(leading_zeros("app.bsky.feed.post/454397e440ec"), 2);
    assert_eq!(leading_zeros("app.bsky.feed.post/9adeb165882c"), 4);
}

pub fn generate_mst(
    db: &mut BlockStore<libipld::DefaultParams>,
    map: &BTreeMap<String, Cid>,
) -> Result<Cid> {
    // construct a "WIP" tree
    let mut root: Option<WipNode> = None;
    for (key, val) in map {
        let height = leading_zeros(key);
        let entry = WipEntry {
            height,
            key: key.clone(),
            val: *val,
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
    let empty_node = WipNode {
        height: 0,
        left: None,
        entries: vec![],
    };
    serialize_wip_tree(db, root.unwrap_or(empty_node))
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
    std::cmp::min(a.len(), b.len())
}

#[test]
fn test_common_prefix_len() {
    assert_eq!(common_prefix_len("abc", "abc"), 3);
    assert_eq!(common_prefix_len("", "abc"), 0);
    assert_eq!(common_prefix_len("abc", ""), 0);
    assert_eq!(common_prefix_len("ab", "abc"), 2);
    assert_eq!(common_prefix_len("abc", "ab"), 2);
    assert_eq!(common_prefix_len("abcde", "abc"), 3);
    assert_eq!(common_prefix_len("abc", "abcde"), 3);
    assert_eq!(common_prefix_len("abcde", "abc1"), 3);
    assert_eq!(common_prefix_len("abcde", "abb"), 2);
    assert_eq!(common_prefix_len("abcde", "qbb"), 0);
    assert_eq!(common_prefix_len("abc", "abc\x00"), 3);
    assert_eq!(common_prefix_len("abc\x00", "abc"), 3);
}

#[test]
fn test_common_prefix_len_wide() {
    // TODO: these are not cross-language consistent!
    assert_eq!("jalapeño".len(), 9); // 8 in javascript
    assert_eq!("💩".len(), 4); // 2 in javascript
    assert_eq!("👩‍👧‍👧".len(), 18); // 8 in javascript

    // many of the below are different in JS; in Rust we *must* cast down to bytes to count
    assert_eq!(common_prefix_len("jalapeño", "jalapeno"), 6);
    assert_eq!(common_prefix_len("jalapeñoA", "jalapeñoB"), 9);
    assert_eq!(common_prefix_len("coöperative", "coüperative"), 3);
    assert_eq!(common_prefix_len("abc💩abc", "abcabc"), 3);
    assert_eq!(common_prefix_len("💩abc", "💩ab"), 6);
    assert_eq!(common_prefix_len("abc👩‍👦‍👦de", "abc👩‍👧‍👧de"), 13);
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
    let cid = *block.cid();
    db.put_block(block, None)?;
    Ok(cid)
}
