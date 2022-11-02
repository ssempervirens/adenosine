
----------- atproto system tables

CREATE TABLE account(
    did                 TEXT PRIMARY KEY NOT NULL,
    username            TEXT NOT NULL,
    email               TEXT NOT NULL,
    password_bcrypt     TEXT NOT NULL,
    signing_key         TEXT NOT NULL
);
CREATE UNIQUE INDEX account_username_uniq_idx on account(lower(username));
CREATE UNIQUE INDEX account_email_uniq_idx on account(lower(email));

CREATE TABLE did_doc(
    did                 TEXT PRIMARY KEY NOT NULL,
    doc_json            TEXT NOT NULL,
    seen_at             TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT ( DATETIME('now') )
);

CREATE TABLE session(
    did                 TEXT NOT NULL,
    jwt                 TEXT NOT NULL,
    created_at          TIMESTAMP WITH TIME ZONE  NOT NULL DEFAULT ( DATETIME('now') ),
    PRIMARY KEY(did, jwt)
);

CREATE TABLE repo(
    did                 TEXT PRIMARY KEY NOT NULL,
    head_commit         TEXT NOT NULL
);

CREATE TABLE record(
    did                 TEXT NOT NULL,
    collection          TEXT NOT NULL,
    tid                 TEXT NOT NULL,
    record_cid          TEXT NOT NULL,
    record_json         TEXT NOT NULL,
    PRIMARY KEY(did, collection, tid)
);

CREATE TABLE password_reset(
    did                 TEXT NOT NULL,
    token               TEXT NOT NULL,
    PRIMARY KEY(did, token)
);

----------- bsky app/index tables

-- TODO
