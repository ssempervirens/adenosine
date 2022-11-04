use crate::load_car_to_blockstore;
use crate::mst::{collect_mst_keys, generate_mst, CommitNode, MetadataNode, RootNode};
use anyhow::{anyhow, ensure, Context, Result};
use ipfs_sqlite_block_store::BlockStore;
use libipld::cbor::DagCborCodec;
use libipld::multihash::Code;
use libipld::prelude::Codec;
use libipld::store::DefaultParams;
use libipld::{Block, Cid, Ipld};
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

    pub fn new_connection(&mut self) -> Result<Self> {
        Ok(RepoStore {
            db: self.db.additional_connection()?,
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
    pub fn put_ipld<S: libipld::codec::Encode<DagCborCodec>>(
        &mut self,
        record: &S,
    ) -> Result<String> {
        let block = Block::<DefaultParams>::encode(DagCborCodec, Code::Sha2_256, record)?;
        let cid = block.cid().clone();
        self.db
            .put_block(block, None)
            .context("writing IPLD DAG-CBOR record to blockstore")?;
        Ok(cid.to_string())
    }

    /// Returns CID that was inserted
    pub fn put_blob(&mut self, data: &[u8]) -> Result<String> {
        let block = Block::<DefaultParams>::encode(libipld::raw::RawCodec, Code::Sha2_256, data)?;
        let cid = block.cid().clone();
        self.db
            .put_block(block, None)
            .context("writing non-record blob to blockstore")?;
        Ok(cid.to_string())
    }

    /// Quick alias lookup
    pub fn lookup_commit(&mut self, did: &str) -> Result<Option<String>> {
        Ok(self
            .db
            .resolve(Cow::from(did.as_bytes()))?
            .map(|cid| cid.to_string()))
    }

    pub fn get_commit(&mut self, commit_cid: &str) -> Result<RepoCommit> {
        // read records by CID: commit, root, meta
        let commit_node: CommitNode = DagCborCodec
            .decode(
                &self
                    .db
                    .get_block(&Cid::from_str(commit_cid)?)?
                    .ok_or(anyhow!("expected commit block in store"))?,
            )
            .context("parsing commit IPLD node from blockstore")?;
        let root_node: RootNode = DagCborCodec
            .decode(
                &self
                    .db
                    .get_block(&commit_node.root)?
                    .ok_or(anyhow!("expected root block in store"))?,
            )
            .context("parsing root IPLD node from blockstore")?;
        let metadata_node: MetadataNode = DagCborCodec
            .decode(
                &self
                    .db
                    .get_block(&root_node.meta)?
                    .ok_or(anyhow!("expected metadata block in store"))?,
            )
            .context("parsing metadata IPLD node from blockstore")?;
        ensure!(
            metadata_node.datastore == "mst",
            "unexpected repo metadata.datastore: {}",
            metadata_node.datastore
        );
        ensure!(
            metadata_node.version == 1,
            "unexpected repo metadata.version: {}",
            metadata_node.version
        );
        Ok(RepoCommit {
            sig: commit_node.sig,
            did: metadata_node.did,
            prev: root_node.prev.map(|cid| cid.to_string()),
            mst_cid: root_node.data.to_string(),
        })
    }

    pub fn get_mst_record_by_key(&mut self, mst_cid: &str, key: &str) -> Result<Option<Ipld>> {
        let map = self.mst_to_map(mst_cid)?;
        if let Some(cid) = map.get(key) {
            self.get_ipld(&cid.to_string()).map(|v| Some(v))
        } else {
            Ok(None)
        }
    }

    pub fn get_atp_record(
        &mut self,
        did: &str,
        collection: &str,
        tid: &str,
    ) -> Result<Option<Ipld>> {
        let commit = if let Some(c) = self.lookup_commit(did)? {
            self.get_commit(&c)?
        } else {
            return Ok(None);
        };
        let record_key = format!("/{}/{}", collection, tid);
        self.get_mst_record_by_key(&commit.mst_cid, &record_key)
    }

    pub fn write_metadata(&mut self, did: &str) -> Result<String> {
        self.put_ipld(&MetadataNode {
            datastore: "mst".to_string(),
            did: did.to_string(),
            version: 1,
        })
    }

    pub fn write_root(
        &mut self,
        meta_cid: &str,
        prev: Option<&str>,
        mst_cid: &str,
    ) -> Result<String> {
        self.put_ipld(&RootNode {
            auth_token: None,
            // TODO: not unwrap here
            prev: prev.map(|s| Cid::from_str(s).unwrap()),
            // TODO: not 'metadata'?
            meta: Cid::from_str(meta_cid)?,
            data: Cid::from_str(mst_cid)?,
        })
    }

    pub fn write_commit(&mut self, did: &str, root_cid: &str, sig: &str) -> Result<String> {
        let commit_cid = self.put_ipld(&CommitNode {
            root: Cid::from_str(root_cid)?,
            sig: sig.as_bytes().to_vec().into_boxed_slice(),
        })?;
        self.db
            .alias(did.as_bytes().to_vec(), Some(&Cid::from_str(&commit_cid)?))?;
        Ok(commit_cid.to_string())
    }

    pub fn mst_from_map(&mut self, map: &BTreeMap<String, String>) -> Result<String> {
        // TODO: not unwrap in iter
        let mut cid_map: BTreeMap<String, Cid> = BTreeMap::from_iter(
            map.iter()
                .map(|(k, v)| (k.to_string(), Cid::from_str(&v).unwrap())),
        );
        let mst_cid = generate_mst(&mut self.db, &mut cid_map)?;
        Ok(mst_cid.to_string())
    }

    fn mst_to_cid_map(&mut self, mst_cid: &str) -> Result<BTreeMap<String, Cid>> {
        let mut cid_map: BTreeMap<String, Cid> = Default::default();
        let mst_cid = Cid::from_str(mst_cid)?;
        collect_mst_keys(&mut self.db, &mst_cid, &mut cid_map)
            .context("reading repo MST from blockstore")?;
        Ok(cid_map)
    }

    /// Returns all the keys for a directory, as a sorted vec of strings
    pub fn mst_to_map(&mut self, mst_cid: &str) -> Result<BTreeMap<String, String>> {
        let cid_map = self.mst_to_cid_map(mst_cid)?;
        let ret_map: BTreeMap<String, String> =
            BTreeMap::from_iter(cid_map.into_iter().map(|(k, v)| (k, v.to_string())));
        Ok(ret_map)
    }

    /// returns the root commit from CAR file
    pub fn load_car(&mut self, car_path: &PathBuf) -> Result<String> {
        let cid = load_car_to_blockstore(&mut self.db, car_path)?;
        Ok(cid.to_string())
    }

    /// Exports in CAR format to a Writer
    ///
    /// The "from" commit CID feature is not implemented.
    pub fn write_car<W: std::io::Write>(
        &mut self,
        _did: &str,
        _from_commit_cid: Option<&str>,
        _out: &mut W,
    ) -> Result<()> {
        unimplemented!()
    }
}

#[test]
fn test_repo_mst() {
    use libipld::ipld;

    let mut repo = RepoStore::open_ephemeral().unwrap();
    let did = "did:plc:dummy";

    // basic blob and IPLD record put/get
    let blob = b"beware the swamp thing";
    let blob_cid: String = repo.put_blob(blob).unwrap();

    let record = ipld!({"some-thing": 123});
    let record_cid: String = repo.put_ipld(&record).unwrap();

    repo.get_blob(&blob_cid).unwrap().unwrap();
    repo.get_ipld(&record_cid).unwrap();

    // basic MST get/put
    let mut map: BTreeMap<String, String> = Default::default();
    let empty_map_cid: String = repo.mst_from_map(&map).unwrap();
    assert_eq!(map, repo.mst_to_map(&empty_map_cid).unwrap());
    assert!(repo
        .get_mst_record_by_key(&empty_map_cid, "/records/1")
        .unwrap()
        .is_none());

    map.insert("/blobs/1".to_string(), blob_cid.clone());
    map.insert("/blobs/2".to_string(), blob_cid.clone());
    map.insert("/records/1".to_string(), record_cid.clone());
    map.insert("/records/2".to_string(), record_cid.clone());
    let simple_map_cid: String = repo.mst_from_map(&map).unwrap();
    assert_eq!(map, repo.mst_to_map(&simple_map_cid).unwrap());

    // create root and commit IPLD nodes
    let meta_cid = repo.write_metadata(did).unwrap();
    let simple_root_cid = repo.write_root(&meta_cid, None, &simple_map_cid).unwrap();
    let simple_commit_cid = repo
        .write_commit(did, &simple_root_cid, "dummy-sig")
        .unwrap();
    assert_eq!(
        Some(record.clone()),
        repo.get_mst_record_by_key(&simple_map_cid, "/records/1")
            .unwrap()
    );
    assert_eq!(
        Some(record.clone()),
        repo.get_atp_record(did, "records", "1").unwrap()
    );
    assert!(repo
        .get_mst_record_by_key(&simple_map_cid, "/records/3")
        .unwrap()
        .is_none());
    assert!(repo.get_atp_record(did, "records", "3").unwrap().is_none());
    assert_eq!(
        Some(simple_commit_cid.clone()),
        repo.lookup_commit(did).unwrap()
    );

    map.insert("/records/3".to_string(), record_cid.clone());
    let simple3_map_cid: String = repo.mst_from_map(&map).unwrap();
    let simple3_root_cid = repo
        .write_root(&meta_cid, Some(&simple_commit_cid), &simple3_map_cid)
        .unwrap();
    let simple3_commit_cid = repo
        .write_commit(did, &simple3_root_cid, "dummy-sig3")
        .unwrap();
    assert_eq!(map, repo.mst_to_map(&simple3_map_cid).unwrap());
    assert_eq!(
        Some(record.clone()),
        repo.get_mst_record_by_key(&simple3_map_cid, "/records/3")
            .unwrap()
    );
    assert_eq!(
        Some(record.clone()),
        repo.get_atp_record(did, "records", "3").unwrap()
    );
    let commit = repo.get_commit(&simple3_commit_cid).unwrap();
    assert_eq!(commit.sig.to_vec(), b"dummy-sig3".to_vec());
    assert_eq!(commit.did, did);
    assert_eq!(commit.prev, Some(simple_commit_cid));
    assert_eq!(commit.mst_cid, simple3_map_cid);
    assert_eq!(
        Some(simple3_commit_cid.clone()),
        repo.lookup_commit(did).unwrap()
    );
}
