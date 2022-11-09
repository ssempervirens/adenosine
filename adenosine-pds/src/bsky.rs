use crate::models::*;
use crate::repo::Mutation;
/// Helper functions for doing database and repo operations relating to bluesky endpoints and
/// records
use crate::{
    ipld_into_json_value, json_value_into_ipld, AtpDatabase, AtpService, Did, Result, Tid,
};
use adenosine_cli::identifiers::{AtUri, Nsid};
use libipld::Cid;
use rusqlite::params;
use serde_json::json;
use std::str::FromStr;

/// Handles updating the database with creation, update, deletion of arbitrary records
pub fn bsky_mutate_db(db: &mut AtpDatabase, did: &Did, mutations: Vec<Mutation>) -> Result<()> {
    // TODO: this function could probably be refactored
    let bsky_post: Nsid = Nsid::from_str("app.bsky.feed.post").unwrap();
    let bsky_repost: Nsid = Nsid::from_str("app.bsky.feed.repost").unwrap();
    let bsky_like: Nsid = Nsid::from_str("app.bsky.feed.like").unwrap();
    let bsky_follow: Nsid = Nsid::from_str("app.bsky.graph.follow").unwrap();
    for m in mutations.into_iter() {
        match m {
            Mutation::Create(ref_type, tid, val) | Mutation::Update(ref_type, tid, val)
                if ref_type == bsky_post =>
            {
                db.bsky_upsert_post(did, &tid, Some(val))?
            }
            Mutation::Delete(ref_type, tid) if ref_type == bsky_post => {
                db.bsky_upsert_post(did, &tid, None)?
            }
            Mutation::Create(ref_type, tid, val) | Mutation::Update(ref_type, tid, val)
                if ref_type == bsky_repost =>
            {
                db.bsky_upsert_ref("repost", did, &tid, Some(val))?
            }
            Mutation::Delete(ref_type, tid) if ref_type == bsky_repost => {
                db.bsky_upsert_ref("repost", did, &tid, None)?
            }
            Mutation::Create(ref_type, tid, val) | Mutation::Update(ref_type, tid, val)
                if ref_type == bsky_like =>
            {
                db.bsky_upsert_ref("like", did, &tid, Some(val))?
            }
            Mutation::Delete(ref_type, tid) if ref_type == bsky_like => {
                db.bsky_upsert_ref("like", did, &tid, None)?
            }
            Mutation::Create(ref_type, tid, val) | Mutation::Update(ref_type, tid, val)
                if ref_type == bsky_follow =>
            {
                db.bsky_upsert_follow(did, &tid, Some(val))?
            }
            Mutation::Delete(ref_type, tid) if ref_type == bsky_follow => {
                db.bsky_upsert_follow(did, &tid, None)?
            }
            _ => (),
        }
    }
    Ok(())
}

pub fn bsky_get_profile(srv: &mut AtpService, did: &Did) -> Result<Profile> {
    // first get the profile record
    let mut profile_cid: Option<Cid> = None;
    let commit_cid = &srv.repo.lookup_commit(&did)?.unwrap();
    let last_commit = srv.repo.get_commit(commit_cid)?;
    let full_map = srv.repo.mst_to_map(&last_commit.mst_cid)?;
    let prefix = "/app.bsky.actor.profile/";
    for (mst_key, cid) in full_map.iter() {
        if mst_key.starts_with(&prefix) {
            profile_cid = Some(*cid);
        }
    }
    let (display_name, description): (Option<String>, Option<String>) =
        if let Some(cid) = profile_cid {
            let record: ProfileRecord =
                serde_json::from_value(ipld_into_json_value(srv.repo.get_ipld(&cid)?))?;
            (Some(record.displayName), record.description)
        } else {
            (None, None)
        };
    let mut stmt = srv
        .atp_db
        .conn
        .prepare_cached("SELECT handle FROM account WHERE did = $1")?;
    let handle: String = stmt.query_row(params!(did.to_string()), |row| row.get(0))?;
    let mut stmt = srv
        .atp_db
        .conn
        .prepare_cached("SELECT COUNT(*) FROM bsky_post WHERE did = $1")?;
    let post_count: u64 = stmt.query_row(params!(did.to_string()), |row| row.get(0))?;
    let mut stmt = srv
        .atp_db
        .conn
        .prepare_cached("SELECT COUNT(*) FROM bsky_ref WHERE ref_type = 'follow' AND did = $1")?;
    let follows_count: u64 = stmt.query_row(params!(did.to_string()), |row| row.get(0))?;
    let uri = format!("at://{}", did);
    let mut stmt = srv
        .atp_db
        .conn
        .prepare_cached("SELECT COUNT(*) FROM bsky_ref WHERE ref_type = 'follow' AND uri = $1")?;
    let followers_count: u64 = stmt.query_row(params!(uri), |row| row.get(0))?;
    Ok(Profile {
        did: did.to_string(),
        handle: handle,
        displayName: display_name,
        description: description,
        followersCount: followers_count,
        followsCount: follows_count,
        postsCount: post_count,
        myState: json!({}),
    })
}

pub fn bsky_update_profile(srv: &mut AtpService, did: &Did, profile: ProfileRecord) -> Result<()> {
    // get the profile record
    let mut profile_tid: Option<Tid> = None;
    let commit_cid = &srv.repo.lookup_commit(&did)?.unwrap();
    let last_commit = srv.repo.get_commit(commit_cid)?;
    let full_map = srv.repo.mst_to_map(&last_commit.mst_cid)?;
    let prefix = "/app.bsky.actor.profile/";
    for (mst_key, _cid) in full_map.iter() {
        if mst_key.starts_with(&prefix) {
            profile_tid = Some(Tid::from_str(mst_key.split('/').nth(1).unwrap())?);
        }
    }
    let profile_tid: Tid = profile_tid.unwrap_or(srv.tid_gen.next_tid());
    let mutations: Vec<Mutation> = vec![Mutation::Update(
        Nsid::from_str("app.bsky.actor.profile")?,
        profile_tid,
        json_value_into_ipld(serde_json::to_value(profile)?),
    )];
    let keypair = srv.pds_keypair.clone();
    srv.repo.mutate_repo(&did, &mutations, &keypair)?;
    Ok(())
}

struct FeedRow {
    pub item_did: Did,
    pub item_handle: String,
    pub item_post_tid: Tid,
    pub item_post_cid: Cid,
}

fn feed_row(row: &rusqlite::Row) -> Result<FeedRow> {
    let item_did: String = row.get(0)?;
    let item_did = Did::from_str(&item_did)?;
    let item_handle = row.get(1)?;
    let item_post_tid: String = row.get(2)?;
    let item_post_tid = Tid::from_str(&item_post_tid)?;
    let cid_string: String = row.get(3)?;
    let item_post_cid = Cid::from_str(&cid_string)?;
    Ok(FeedRow {
        item_did,
        item_handle,
        item_post_tid,
        item_post_cid,
    })
}

fn feed_row_to_item(srv: &mut AtpService, row: FeedRow) -> Result<FeedItem> {
    let record_ipld = srv.repo.get_ipld(&row.item_post_cid)?;
    let feed_item = FeedItem {
        uri: format!(
            "at://{}/{}/{}",
            row.item_did, "app.bsky.feed.post", row.item_post_tid
        ),
        cid: row.item_post_cid.to_string(),
        author: User {
            did: row.item_did.to_string(),
            handle: row.item_handle,
            displayName: None, // TODO
        },
        repostedBy: None,
        record: ipld_into_json_value(record_ipld),
        embed: None,
        replyCount: 0,                 // TODO
        repostCount: 0,                // TODO
        likeCount: 0,                  // TODO
        indexedAt: "TODO".to_string(), // TODO
        myState: None,
    };
    Ok(feed_item)
}

pub fn bsky_get_author_feed(srv: &mut AtpService, did: &Did) -> Result<GenericFeed> {
    let mut feed: Vec<FeedItem> = vec![];
    let rows = {
        let mut stmt = srv.atp_db
            .conn
            .prepare_cached("SELECT account.did, account.handle, bsky_post.tid, bsky_post.cid, FROM bsky_post LEFT JOIN account ON bsky_post.did = account.did LEFT JOIN bsky_follow ON bsky_post.did = bsky_follow.subject_did WHERE bsky_follow.did = ?1 ORDER BY bsky_post.tid DESC LIMIT 20")?;
        let mut sql_rows = stmt.query(params!(did.to_string()))?;
        let mut rows: Vec<FeedRow> = vec![];
        while let Some(sql_row) = sql_rows.next()? {
            let row = feed_row(sql_row)?;
            rows.push(row);
        }
        rows
    };
    for row in rows {
        feed.push(feed_row_to_item(srv, row)?);
    }
    Ok(GenericFeed { feed })
}

pub fn bsky_get_timeline(srv: &mut AtpService, did: &Did) -> Result<GenericFeed> {
    let mut feed: Vec<FeedItem> = vec![];
    let rows = {
        let mut stmt = srv.atp_db
            .conn
            .prepare_cached("SELECT account.did, account.handle, bsky_post.tid, bsky_post.cid, FROM bsky_post LEFT JOIN account ON bsky_post.did = account.did WHERE bsky_post.did = ?1 ORDER BY bsky_post.tid DESC LIMIT 20")?;
        let mut sql_rows = stmt.query(params!(did.to_string()))?;
        let mut rows: Vec<FeedRow> = vec![];
        while let Some(sql_row) = sql_rows.next()? {
            let row = feed_row(sql_row)?;
            rows.push(row);
        }
        rows
    };
    for row in rows {
        feed.push(feed_row_to_item(srv, row)?);
    }
    Ok(GenericFeed { feed })
}

pub fn bsky_get_thread(
    _srv: &mut AtpService,
    _uri: &AtUri,
    _depth: Option<u64>,
) -> Result<GenericFeed> {
    // TODO: what is the best way to implement this? recurisvely? just first-level children to
    // start?
    unimplemented!()
}
