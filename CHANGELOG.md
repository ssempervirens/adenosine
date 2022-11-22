
# CHANGELOG

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
