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
        .prepare_cached("SELECT COUNT(*) FROM bsky_follow WHERE did = $1")?;
    let follows_count: u64 = stmt.query_row(params!(did.to_string()), |row| row.get(0))?;
    let mut stmt = srv
        .atp_db
        .conn
        .prepare_cached("SELECT COUNT(*) FROM bsky_follow WHERE subject_did = $1")?;
    let followers_count: u64 = stmt.query_row(params!(did.to_string()), |row| row.get(0))?;
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
            profile_tid = Some(Tid::from_str(mst_key.split('/').nth(2).unwrap())?);
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
    pub indexed_at: String,
}

fn feed_row(row: &rusqlite::Row) -> Result<FeedRow> {
    let item_did: String = row.get(0)?;
    let item_did = Did::from_str(&item_did)?;
    let item_handle = row.get(1)?;
    let item_post_tid: String = row.get(2)?;
    let item_post_tid = Tid::from_str(&item_post_tid)?;
    let cid_string: String = row.get(3)?;
    let item_post_cid = Cid::from_str(&cid_string)?;
    let indexed_at: String = row.get(4)?;
    Ok(FeedRow {
        item_did,
        item_handle,
        item_post_tid,
        item_post_cid,
        indexed_at,
    })
}

fn feed_row_to_item(srv: &mut AtpService, row: FeedRow) -> Result<FeedItem> {
    let record_ipld = srv.repo.get_ipld(&row.item_post_cid)?;
    let uri = format!(
        "at://{}/{}/{}",
        row.item_did, "app.bsky.feed.post", row.item_post_tid
    );

    let mut stmt = srv.atp_db.conn.prepare_cached(
        "SELECT COUNT(*) FROM bsky_ref WHERE ref_type = 'like' AND subject_uri = $1",
    )?;
    let like_count: u64 = stmt.query_row(params!(uri), |row| row.get(0))?;

    let mut stmt = srv.atp_db.conn.prepare_cached(
        "SELECT COUNT(*) FROM bsky_ref WHERE ref_type = 'repost' AND subject_uri = $1",
    )?;
    let repost_count: u64 = stmt.query_row(params!(uri), |row| row.get(0))?;

    let mut stmt = srv
        .atp_db
        .conn
        .prepare_cached("SELECT COUNT(*) FROM bsky_post WHERE reply_to_parent_uri = $1")?;
    let reply_count: u64 = stmt.query_row(params!(uri), |row| row.get(0))?;

    let feed_item = FeedItem {
        uri: uri,
        cid: Some(row.item_post_cid.to_string()),
        author: User {
            did: row.item_did.to_string(),
            handle: row.item_handle,
            displayName: None, // TODO: fetch from profile (or cache)
        },
        repostedBy: None,
        record: ipld_into_json_value(record_ipld),
        embed: None,
        replyCount: reply_count,
        repostCount: repost_count,
        likeCount: like_count,
        indexedAt: row.indexed_at,
        myState: None,
    };
    Ok(feed_item)
}

pub fn bsky_get_timeline(srv: &mut AtpService, did: &Did) -> Result<GenericFeed> {
    let mut feed: Vec<FeedItem> = vec![];
    // TODO: also handle reposts
    let rows = {
        let mut stmt = srv.atp_db
            .conn
            .prepare_cached("SELECT account.did, account.handle, bsky_post.tid, bsky_post.cid, bsky_post.indexed_at FROM bsky_post LEFT JOIN account ON bsky_post.did = account.did LEFT JOIN bsky_follow ON bsky_post.did = bsky_follow.subject_did WHERE bsky_follow.did = ?1 AND account.did IS NOT NULL ORDER BY bsky_post.tid DESC LIMIT 20")?;
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

pub fn bsky_get_author_feed(srv: &mut AtpService, did: &Did) -> Result<GenericFeed> {
    let mut feed: Vec<FeedItem> = vec![];
    // TODO: also handle reposts
    let rows = {
        let mut stmt = srv.atp_db
            .conn
            .prepare_cached("SELECT account.did, account.handle, bsky_post.tid, bsky_post.cid, bsky_post.indexed_at FROM bsky_post LEFT JOIN account ON bsky_post.did = account.did WHERE bsky_post.did = ?1 ORDER BY bsky_post.tid DESC LIMIT 20")?;
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

#[test]
fn test_bsky_profile() {
    use crate::{create_account, created_at_now};
    use libipld::ipld;

    let post_nsid = Nsid::from_str("app.bsky.feed.post").unwrap();
    let follow_nsid = Nsid::from_str("app.bsky.graph.follow").unwrap();

    let mut srv = AtpService::new_ephemeral().unwrap();
    let req = AccountRequest {
        email: "test@bogus.com".to_string(),
        handle: "handle.test".to_string(),
        password: "bogus".to_string(),
        inviteCode: None,
        recoveryKey: None,
    };
    let session = create_account(&mut srv, &req, true).unwrap();
    let did = Did::from_str(&session.did).unwrap();
    let profile = bsky_get_profile(&mut srv, &did).unwrap();
    assert_eq!(profile.did, session.did);
    assert_eq!(profile.handle, req.handle);
    assert_eq!(profile.displayName, None);
    assert_eq!(profile.description, None);
    assert_eq!(profile.followersCount, 0);
    assert_eq!(profile.followsCount, 0);
    assert_eq!(profile.postsCount, 0);

    let record = ProfileRecord {
        displayName: "Test Name".to_string(),
        description: Some("short description".to_string()),
    };
    bsky_update_profile(&mut srv, &did, record.clone()).unwrap();
    let profile = bsky_get_profile(&mut srv, &did).unwrap();
    assert_eq!(profile.displayName, Some(record.displayName));
    assert_eq!(profile.description, record.description);

    let record = ProfileRecord {
        displayName: "New Test Name".to_string(),
        description: Some("longer description".to_string()),
    };
    bsky_update_profile(&mut srv, &did, record.clone()).unwrap();
    let profile = bsky_get_profile(&mut srv, &did).unwrap();
    assert_eq!(profile.displayName, Some(record.displayName));
    assert_eq!(profile.description, record.description);

    let mutations = vec![
        Mutation::Create(
            follow_nsid.clone(),
            srv.tid_gen.next_tid(),
            ipld!({"subject": {"did": session.did}, "createdAt": created_at_now()}),
        ),
        Mutation::Create(
            follow_nsid.clone(),
            srv.tid_gen.next_tid(),
            ipld!({"subject": {"did": "did:web:external.domain"}, "createdAt": created_at_now()}),
        ),
        Mutation::Create(
            post_nsid.clone(),
            srv.tid_gen.next_tid(),
            ipld!({"text": "first post"}),
        ),
        Mutation::Create(
            post_nsid.clone(),
            srv.tid_gen.next_tid(),
            ipld!({"text": "second post"}),
        ),
        Mutation::Create(
            post_nsid.clone(),
            srv.tid_gen.next_tid(),
            ipld!({"text": "third post"}),
        ),
    ];
    srv.repo
        .mutate_repo(&did, &mutations, &srv.pds_keypair)
        .unwrap();
    bsky_mutate_db(&mut srv.atp_db, &did, mutations).unwrap();

    let profile = bsky_get_profile(&mut srv, &did).unwrap();
    assert_eq!(profile.followersCount, 1);
    assert_eq!(profile.followsCount, 2);
    assert_eq!(profile.postsCount, 3);
}

#[test]
fn test_bsky_feeds() {
    use crate::{create_account, created_at_now};
    use libipld::ipld;

    let post_nsid = Nsid::from_str("app.bsky.feed.post").unwrap();
    let like_nsid = Nsid::from_str("app.bsky.feed.like").unwrap();
    let repost_nsid = Nsid::from_str("app.bsky.feed.repost").unwrap();
    let follow_nsid = Nsid::from_str("app.bsky.graph.follow").unwrap();

    let mut srv = AtpService::new_ephemeral().unwrap();
    let alice_did = {
        let req = AccountRequest {
            email: "alice@bogus.com".to_string(),
            handle: "alice.test".to_string(),
            password: "bogus".to_string(),
            inviteCode: None,
            recoveryKey: None,
        };
        let session = create_account(&mut srv, &req, true).unwrap();
        Did::from_str(&session.did).unwrap()
    };
    let bob_did = {
        let req = AccountRequest {
            email: "bob@bogus.com".to_string(),
            handle: "bob.test".to_string(),
            password: "bogus".to_string(),
            inviteCode: None,
            recoveryKey: None,
        };
        let session = create_account(&mut srv, &req, true).unwrap();
        Did::from_str(&session.did).unwrap()
    };
    let carol_did = {
        let req = AccountRequest {
            email: "carol@bogus.com".to_string(),
            handle: "carol.test".to_string(),
            password: "bogus".to_string(),
            inviteCode: None,
            recoveryKey: None,
        };
        let session = create_account(&mut srv, &req, true).unwrap();
        Did::from_str(&session.did).unwrap()
    };

    // all feeds and timelines should be empty
    let alice_feed = bsky_get_author_feed(&mut srv, &alice_did).unwrap();
    let alice_timeline = bsky_get_timeline(&mut srv, &alice_did).unwrap();
    assert!(alice_feed.feed.is_empty());
    assert!(alice_timeline.feed.is_empty());
    let bob_feed = bsky_get_author_feed(&mut srv, &bob_did).unwrap();
    let bob_timeline = bsky_get_timeline(&mut srv, &bob_did).unwrap();
    assert!(bob_feed.feed.is_empty());
    assert!(bob_timeline.feed.is_empty());
    let carol_feed = bsky_get_author_feed(&mut srv, &carol_did).unwrap();
    let carol_timeline = bsky_get_timeline(&mut srv, &carol_did).unwrap();
    assert!(carol_feed.feed.is_empty());
    assert!(carol_timeline.feed.is_empty());

    // alice does some posts
    let alice_post1_tid = srv.tid_gen.next_tid();
    let alice_post2_tid = srv.tid_gen.next_tid();
    let alice_post3_tid = srv.tid_gen.next_tid();
    assert!(alice_post1_tid < alice_post2_tid && alice_post2_tid < alice_post3_tid);
    let mutations = vec![
        Mutation::Create(
            post_nsid.clone(),
            alice_post1_tid.clone(),
            ipld!({"text": "alice first post"}),
        ),
        Mutation::Create(
            post_nsid.clone(),
            alice_post2_tid.clone(),
            ipld!({"text": "alice second post"}),
        ),
        Mutation::Create(
            post_nsid.clone(),
            alice_post3_tid.clone(),
            ipld!({"text": "alice third post"}),
        ),
    ];
    srv.repo
        .mutate_repo(&alice_did, &mutations, &srv.pds_keypair)
        .unwrap();
    bsky_mutate_db(&mut srv.atp_db, &alice_did, mutations).unwrap();

    // bob follows alice, likes first post, reposts second, replies third
    let alice_post3_uri = format!(
        "at://{}/{}/{}",
        alice_did.to_string(),
        post_nsid.to_string(),
        alice_post3_tid.to_string()
    );
    let mutations = vec![
        Mutation::Create(
            follow_nsid.clone(),
            srv.tid_gen.next_tid(),
            ipld!({"subject": {"did": alice_did.to_string()}, "createdAt": created_at_now()}),
        ),
        Mutation::Create(
            like_nsid.clone(),
            srv.tid_gen.next_tid(),
            ipld!({"subject": {"uri": format!("at://{}/{}/{}", alice_did.to_string(), post_nsid.to_string(), alice_post1_tid.to_string())}, "createdAt": created_at_now()}),
        ),
        Mutation::Create(
            repost_nsid.clone(),
            srv.tid_gen.next_tid(),
            ipld!({"subject": {"uri": format!("at://{}/{}/{}", alice_did.to_string(), post_nsid.to_string(), alice_post2_tid.to_string())}, "createdAt": created_at_now()}),
        ),
        Mutation::Create(
            post_nsid.clone(),
            srv.tid_gen.next_tid(),
            ipld!({"text": "bob comment on alice post3", "reply": {"parent": {"uri": alice_post3_uri.clone()}, "root": {"uri": alice_post3_uri.clone()}}}),
        ),
    ];
    srv.repo
        .mutate_repo(&bob_did, &mutations, &srv.pds_keypair)
        .unwrap();
    bsky_mutate_db(&mut srv.atp_db, &bob_did, mutations).unwrap();

    // carol follows bob
    let mutations = vec![Mutation::Create(
        follow_nsid.clone(),
        srv.tid_gen.next_tid(),
        ipld!({"subject": {"did": bob_did.to_string()}, "createdAt": created_at_now()}),
    )];
    srv.repo
        .mutate_repo(&bob_did, &mutations, &srv.pds_keypair)
        .unwrap();
    bsky_mutate_db(&mut srv.atp_db, &carol_did, mutations).unwrap();

    // test alice profile: counts should be updated
    let alice_profile = bsky_get_profile(&mut srv, &alice_did).unwrap();
    assert_eq!(alice_profile.followersCount, 1);
    assert_eq!(alice_profile.followsCount, 0);
    assert_eq!(alice_profile.postsCount, 3);

    // test alice timeline: still empty (?)
    let alice_timeline = bsky_get_timeline(&mut srv, &alice_did).unwrap();
    println!("{:?}", alice_timeline);
    assert!(alice_timeline.feed.is_empty());

    // test alice feed: should have 3 posts, with correct counts
    let alice_feed = bsky_get_author_feed(&mut srv, &alice_did).unwrap();
    assert_eq!(alice_feed.feed.len(), 3);

    assert_eq!(
        alice_feed.feed[2].uri,
        format!(
            "at://{}/{}/{}",
            alice_did.to_string(),
            post_nsid.to_string(),
            alice_post1_tid.to_string()
        )
    );
    // TODO: CID
    assert_eq!(alice_feed.feed[2].author.did, alice_did.to_string());
    assert_eq!(alice_feed.feed[2].author.handle, "alice.test");
    assert_eq!(alice_feed.feed[2].repostedBy, None);
    assert_eq!(
        alice_feed.feed[2].record["text"].as_str().unwrap(),
        "alice first post"
    );
    assert_eq!(alice_feed.feed[2].embed, None);
    assert_eq!(alice_feed.feed[2].replyCount, 0);
    assert_eq!(alice_feed.feed[2].repostCount, 0);
    assert_eq!(alice_feed.feed[2].likeCount, 1);

    assert_eq!(alice_feed.feed[1].author.did, alice_did.to_string());
    assert_eq!(alice_feed.feed[1].replyCount, 0);
    assert_eq!(alice_feed.feed[1].repostCount, 1);
    assert_eq!(alice_feed.feed[1].likeCount, 0);

    assert_eq!(alice_feed.feed[0].author.did, alice_did.to_string());
    assert_eq!(alice_feed.feed[0].replyCount, 1);
    assert_eq!(alice_feed.feed[0].repostCount, 0);
    assert_eq!(alice_feed.feed[0].likeCount, 0);

    // test bob timeline: should include alice posts
    let bob_timeline = bsky_get_timeline(&mut srv, &bob_did).unwrap();
    println!("BOB TIMELINE ======");
    for item in bob_timeline.feed.iter() {
        println!("{:?}", item);
    }
    assert_eq!(bob_timeline.feed.len(), 3);
    assert_eq!(
        bob_timeline.feed[2].uri,
        format!(
            "at://{}/{}/{}",
            alice_did.to_string(),
            post_nsid.to_string(),
            alice_post1_tid.to_string()
        )
    );
    // TODO: CID
    assert_eq!(bob_timeline.feed[2].author.did, alice_did.to_string());
    assert_eq!(bob_timeline.feed[2].author.handle, "alice.test");

    // test bob feed: should include repost and reply
    let bob_feed = bsky_get_author_feed(&mut srv, &bob_did).unwrap();
    assert_eq!(bob_feed.feed.len(), 1);
    // TODO: handle reposts
    /*
    assert_eq!(bob_feed.feed.len(), 2);
    assert_eq!(bob_feed.feed[1].uri, format!("at://{}/{}/{}", alice_did.to_string(), post_nsid.to_string(), alice_post1_tid.to_string()));
    // TODO: CID
    assert_eq!(bob_feed.feed[1].author.did, alice_did.to_string());
    assert_eq!(bob_feed.feed[1].author.handle, "alice.test");
    assert_eq!(bob_feed.feed[1].repostedBy.as_ref().unwrap().did, bob_did.to_string());
    assert_eq!(bob_feed.feed[1].repostedBy.as_ref().unwrap().handle, "bob.test");
    // TODO: "is a repost" (check record?)
    */

    assert_eq!(bob_feed.feed[0].author.did, bob_did.to_string());
    assert_eq!(bob_feed.feed[0].author.handle, "bob.test");

    // test carol timeline: should include bob's repost and reply
    let carol_timeline = bsky_get_timeline(&mut srv, &carol_did).unwrap();
    // TODO: handle re-posts (+1 here)
    assert_eq!(carol_timeline.feed.len(), 1);
    // TODO: details

    // test carol feed: still empty
    let carol_feed = bsky_get_author_feed(&mut srv, &carol_did).unwrap();
    assert!(carol_feed.feed.is_empty());
}

// TODO: post threads (include in the above test?)