
# CHANGELOG

## [0.3.0] - UNRELEASED

Refactored common library code into `adenosine` crate. Will put common types,
helpers, and probably client code and generated Lexicon code there.

## Changed

- mst: include "empty" intermediate nodes between layers (following upstream
  behavior)

## Added
- mst: interop tests with upstream `atproto` (Typescript) repository

## [0.2.0] - 2022-12-19

Tracking upstream Lexicon changes. Not backwards-compatible at the CLI/PDS XRPC
layer, but doesn't seem to impact existing repo content. Bumping major version
for this reason.

The PDS sqlite schema was tweaked, but only to change the auto-generated
`indexed_at` columns to use milisecond precision; believe this should be
backwards/forwards compatible.

Still not a complete implementation of lexicons, in either CLI or PDS. Notably,
neither supports bsky media attachments (blobs, photos, etc) in this release.

## Changed

- both: most XRPC POST params moved from HTTP query to JSON body (upstream lexicon change)
- both: use milisecond precision in timestamps (eg, `createdAt`)
- pds: use milisecond precision in sqlite-generated `indexedAt` fields (schema change)
- update deps (`Cargo.lock`)
- both: `getActor` query param is "actor" not "user" (so far upstream has not
  refactored other "user" instances)
- pds: udpated `getProfile` schema
- pds: "upvotesCount" in schemas, not "likesCount"

## Added

- both: com.atproto.session.refresh endpoint
- pds: stub implementation of new `app.bsky.actor` lexicons (`search`,
  `searchTypeahead`, `getSuggestions`), notifications, memberships

## Fixed

- pds: `getProfile` for non-existant DID as an error, not panic


## [0.1.2] - 2022-11-22

## Added

- pds: CAR download links

## Changed

- both: proper timestamp formatting, both in Rust and SQL schema
- both: clippy lint fixes
- both: TID formatting and generation
- pds: cleaner commit metadata display in web UI


## [0.1.1] - 2022-11-11

## Fixed

- crate-specific README files included in crate metadata

## Changed

- all: `dotenv` replace by `dotenvy`
- pds: pink inspect links

## [0.1.0] - 2022-11-11

First tagged release.

Both the AT protocol and this project are very much a work in progress, and
there should be zero expectation of stability, backwards/forwards comatability,
or supported upgrade paths at this time.

Initial features:

- cli: blocking implementation with `reqwest`
- cli: generic XRPC methods (com.atproto Lexicon)
- cli: partial app.bsky Lexicon support
- pds: blocking implementation with `rouille`, `rusqlite`, `ipfs-sqlite-block-store`
- pds: crude repository storage (MST in IPLD blocks)
- pds: crude system and application database (sqlite)
- pds: basic read-only web interface to repository content, bsky profiles and posts
- pds: self-hosted did:web configuration
- pds: small-world did:plc with registration configuration
