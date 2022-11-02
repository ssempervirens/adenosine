use crate::mst::{collect_mst_keys, CommitNode, MetadataNode, RootNode};
use anyhow::{anyhow, Result};
use ipfs_sqlite_block_store::BlockStore;
use libipld::cbor::DagCborCodec;
use libipld::multihash::Code;
use libipld::prelude::Codec;
use libipld::store::DefaultParams;
use libipld::{Block, Cid, DagCbor, Ipld};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;

pub struct RepoCommit {
    pub sig: Box<[u8]>,
    pub did: String,
    pub prev: Option<String>,
    pub mst_cid: String,
}

pub struct RepoStore {
    db: BlockStore<libipld::DefaultParams>,
}

impl RepoStore {
    pub fn open(db_path: &PathBuf) -> Result<Self> {
        Ok(RepoStore {
            db: BlockStore::open(db_path, Default::default())?,
        })
    }

    pub fn open_ephemeral() -> Result<Self> {
        Ok(RepoStore {
            db: BlockStore::open_path(ipfs_sqlite_block_store::DbPath::Memory, Default::default())?,
        })
    }

    pub fn get_ipld(&mut self, cid: &str) -> Result<Ipld> {
        let ipld_cid = Cid::from_str(cid)?;
        if let Some(b) = self.db.get_block(&ipld_cid)? {
            let block: Block<DefaultParams> = Block::new(ipld_cid, b)?;
            block.ipld()
        } else {
            Err(anyhow!("missing IPLD CID: {}", cid))
        }
    }

    pub fn get_blob(&mut self, cid: &str) -> Result<Option<Vec<u8>>> {
        let cid = Cid::from_str(cid)?;
        Ok(self.db.get_block(&cid)?)
    }

    /// Returns CID that was inserted
    pub fn put_ipld(&mut self, record: &Ipld) -> Result<String> {
        let block = Block::<DefaultParams>::encode(DagCborCodec, Code::Sha2_256, record)?;
        let cid = block.cid().clone();
        self.db.put_block(block, None)?;
        Ok(cid.to_string())
    }

    /// Returns CID that was inserted
    pub fn put_blob(&mut self, data: Vec<u8>) -> Result<String> {
        let block = Block::<DefaultParams>::encode(libipld::raw::RawCodec, Code::Sha2_256, &data)?;
        let cid = block.cid().clone();
        self.db.put_block(block, None)?;
        Ok(cid.to_string())
    }

    /// Quick alias lookup
    pub fn get_root(&mut self, did: &str) -> Result<Option<String>> {
        Ok(self
            .db
            .resolve(Cow::from(did.as_bytes()))?
            .map(|cid| cid.to_string()))
    }

    pub fn get_commit(&mut self, commit_cid: &str) -> Result<RepoCommit> {
        // read records by CID: commit, root, meta
        let commit_node: CommitNode = DagCborCodec.decode(
            &self
                .db
                .get_block(&Cid::from_str(commit_cid)?)?
                .ok_or(anyhow!("expected commit block in store"))?,
        )?;
        let root_node: RootNode = DagCborCodec.decode(
            &self
                .db
                .get_block(&commit_node.root)?
                .ok_or(anyhow!("expected root block in store"))?,
        )?;
        let metadata_node: MetadataNode = DagCborCodec.decode(
            &self
                .db
                .get_block(&root_node.meta)?
                .ok_or(anyhow!("expected metadata block in store"))?,
        )?;
        assert_eq!(metadata_node.datastore, "mst");
        assert_eq!(metadata_node.version, 1);
        Ok(RepoCommit {
            sig: commit_node.sig,
            did: metadata_node.did,
            prev: root_node.prev.map(|cid| cid.to_string()),
            mst_cid: root_node.data.to_string(),
        })
    }

    pub fn get_record_by_key(&mut self, commit_cid: &str, key: &str) -> Result<Option<Ipld>> {
        let map = self.as_map(commit_cid)?;
        if let Some(cid) = map.get(key) {
            self.get_ipld(&cid.to_string()).map(|v| Some(v))
        } else {
            Ok(None)
        }
    }

    pub fn write_root(&mut self, did: &str, mst_cid: &str, prev: Option<&str>) -> Result<String> {
        unimplemented!()
    }

    pub fn write_commit(&mut self, did: &str, root_cid: &str, sig: &str) -> Result<String> {
        // TODO: also update alias to point to new commit
        unimplemented!()
    }

    pub fn write_map(&self, map: Result<BTreeMap<String, String>>) -> Result<String> {
        unimplemented!()
    }

    fn as_cid_map(&mut self, commit_cid: &str) -> Result<BTreeMap<String, Cid>> {
        let commit = self.get_commit(commit_cid)?;
        let mut cid_map: BTreeMap<String, Cid> = Default::default();
        let mst_cid = Cid::from_str(&commit.mst_cid)?;
        collect_mst_keys(&mut self.db, &mst_cid, &mut cid_map)?;
        Ok(cid_map)
    }

    /// Returns all the keys for a directory, as a sorted vec of strings
    pub fn as_map(&mut self, commit_cid: &str) -> Result<BTreeMap<String, String>> {
        let cid_map = self.as_cid_map(commit_cid)?;
        let ret_map: BTreeMap<String, String> =
            BTreeMap::from_iter(cid_map.into_iter().map(|(k, v)| (k, v.to_string())));
        Ok(ret_map)
    }

    /// Exports in CAR format to a Writer
    ///
    /// The "from" commit CID feature is not implemented.
    pub fn write_car<W: std::io::Write>(
        &mut self,
        did: &str,
        _from_commit_cid: Option<&str>,
        out: &mut W,
    ) -> Result<()> {
        unimplemented!()
    }
}
