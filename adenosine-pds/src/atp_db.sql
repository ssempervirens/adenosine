
----------- atproto PDS system tables

CREATE TABLE account(
    did                 TEXT PRIMARY KEY NOT NULL,
    handle              TEXT NOT NULL,
    email               TEXT NOT NULL,
    password_bcrypt     TEXT NOT NULL,
    recovery_pubkey     TEXT NOT NULL,
    created_at          TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%fZ', 'now') )
);
CREATE UNIQUE INDEX account_handle_uniq_idx on account(lower(handle));
CREATE UNIQUE INDEX account_email_uniq_idx on account(lower(email));

CREATE TABLE did_doc(
    did                 TEXT PRIMARY KEY NOT NULL,
    -- TODO: handle              TEXT NOT NULL,
    doc_json            TEXT NOT NULL,
    indexed_at          TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%fZ', 'now') )
);

CREATE TABLE session(
    did                 TEXT NOT NULL,
    jwt                 TEXT NOT NULL,
    created_at          TIMESTAMP WITH TIME ZONE  NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%fZ', 'now') ),
    PRIMARY KEY(did, jwt)
);

----------- bsky app/index tables

CREATE TABLE bsky_post(
    did                 TEXT NOT NULL,
    tid                 TEXT NOT NULL,
    cid                 TEXT NOT NULL,
    record_json         TEXT NOT NULL,
    reply_to_parent_uri    TEXT,
    reply_to_root_uri      TEXT,
    created_at          TIMESTAMP WITH TIME ZONE  NOT NULL,
    indexed_at          TIMESTAMP WITH TIME ZONE  NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%fZ', 'now') ),
    PRIMARY KEY(did, tid)
);
CREATE INDEX bsky_post_reply_to_parent_uri_idx on bsky_post(reply_to_parent_uri);
CREATE INDEX bsky_post_reply_to_root_uri_idx on bsky_post(reply_to_root_uri);

CREATE TABLE bsky_ref(
    ref_type            TEXT NOT NULL,
    did                 TEXT NOT NULL,
    tid                 TEXT NOT NULL,
    subject_uri         TEXT NOT NULL,
    -- TODO: NOT NULL on subject_cid
    subject_cid         TEXT,
    created_at          TIMESTAMP WITH TIME ZONE  NOT NULL,
    indexed_at          TIMESTAMP WITH TIME ZONE  NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%fZ', 'now') ),
    PRIMARY KEY(ref_type, did, tid)
);
CREATE INDEX bsky_ref_subject_uri_idx on bsky_ref(subject_uri);

CREATE TABLE bsky_follow(
    did                 TEXT NOT NULL,
    tid                 TEXT NOT NULL,
    subject_did         TEXT NOT NULL,
    -- TODO: NOT NULL on subject_cid
    subject_cid         TEXT,
    created_at          TIMESTAMP WITH TIME ZONE  NOT NULL,
    indexed_at          TIMESTAMP WITH TIME ZONE  NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%fZ', 'now') ),
    PRIMARY KEY(did, tid)
);
CREATE INDEX bsky_follow_subject_did_idx on bsky_follow(subject_did);

-- TODO: notifications
CREATE TABLE bsky_notification(
    pk                  INTEGER PRIMARY KEY AUTOINCREMENT,
    user_did            TEXT NOT NULL,
    subject_uri         TEXT NOT NULL,
    -- TODO: NOT NULL on subject_cid
    subject_cid         TEXT,
    reason              TEXT NOT NULL,
    seen_at             TIMESTAMP WITH TIME ZONE,
    indexed_at          TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%fZ', 'now') )
);
