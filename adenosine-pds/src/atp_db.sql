
----------- atproto PDS system tables

CREATE TABLE account(
    did                 TEXT PRIMARY KEY NOT NULL,
    username            TEXT NOT NULL,
    email               TEXT NOT NULL,
    password_bcrypt     TEXT NOT NULL,
    recovery_pubkey     TEXT NOT NULL,
    created_at          TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT ( DATETIME('now') )
);
CREATE UNIQUE INDEX account_username_uniq_idx on account(lower(username));
CREATE UNIQUE INDEX account_email_uniq_idx on account(lower(email));

CREATE TABLE did_doc(
    did                 TEXT PRIMARY KEY NOT NULL,
    -- TODO: username            TEXT NOT NULL,
    doc_json            TEXT NOT NULL,
    indexed_at          TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT ( DATETIME('now') )
);

CREATE TABLE session(
    did                 TEXT NOT NULL,
    jwt                 TEXT NOT NULL,
    created_at          TIMESTAMP WITH TIME ZONE  NOT NULL DEFAULT ( DATETIME('now') ),
    PRIMARY KEY(did, jwt)
);

----------- bsky app/index tables

CREATE TABLE bsky_post(
    did                 TEXT NOT NULL,
    tid                 TEXT NOT NULL,
    cid                 TEXT NOT NULL,
    record_json         TEXT NOT NULL,
    reply_root_uri      TEXT,
    created_at          TIMESTAMP WITH TIME ZONE  NOT NULL,
    indexed_at          TIMESTAMP WITH TIME ZONE  NOT NULL DEFAULT ( DATETIME('now') ),
    PRIMARY KEY(did, tid)
);
CREATE INDEX bsky_post_reply_root_uri_idx on bsky_post(reply_root_uri);

CREATE TABLE bsky_repost(
    did                 TEXT NOT NULL,
    subject_uri         TEXT NOT NULL,
    cid                 TEXT NOT NULL,
    created_at          TIMESTAMP WITH TIME ZONE  NOT NULL,
    indexed_at          TIMESTAMP WITH TIME ZONE  NOT NULL DEFAULT ( DATETIME('now') ),
    PRIMARY KEY(did, subject_uri)
);
CREATE INDEX bsky_repost_subject_uri_idx on bsky_repost(subject_uri);

CREATE TABLE bsky_like(
    did                 TEXT NOT NULL,
    subject_uri         TEXT NOT NULL,
    cid                 TEXT NOT NULL,
    created_at          TIMESTAMP WITH TIME ZONE  NOT NULL,
    indexed_at          TIMESTAMP WITH TIME ZONE  NOT NULL DEFAULT ( DATETIME('now') ),
    PRIMARY KEY(did, subject_uri)
);
CREATE INDEX bsky_like_subject_uri_idx on bsky_like(subject_uri);

CREATE TABLE bsky_follow(
    did                 TEXT NOT NULL,
    subject_did         TEXT NOT NULL,
    cid                 TEXT NOT NULL,
    created_at          TIMESTAMP WITH TIME ZONE  NOT NULL,
    indexed_at          TIMESTAMP WITH TIME ZONE  NOT NULL DEFAULT ( DATETIME('now') ),
    PRIMARY KEY(did, subject_did)
);
CREATE INDEX bsky_follow_subject_did_idx on bsky_follow(subject_did);

-- TODO: notifications
CREATE TABLE bsky_notification(
    pk                  INTEGER PRIMARY KEY AUTOINCREMENT,
    user_did            TEXT NOT NULL,
    subject_uri         TEXT NOT NULL,
    subject_cid         TEXT NOT NULL,
    reason              TEXT NOT NULL,
    seen_at             TIMESTAMP WITH TIME ZONE,
    indexed_at          TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT ( DATETIME('now') )
);
