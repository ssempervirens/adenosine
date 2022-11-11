
`adenosine-pds`: small-world atproto.com Personal Data Server
=============================================================

This is a simple, enthusiast-grade AT Protocol
([atproto.com](https://atproto.com)) personal data server ("PDS")
implementation. It targets "small-world" uses cases of the protocol, for
example personal or organizational self-hosting.

There is a [manpage](../extra/adenosine-pds.1.md) with usage and configuration
details.

Features:

- [x] com.atproto Lexicon implementation (not validated)
- [x] crude repo CAR file export (full history)
- [x] partial app.bsky Lexicon implementation (posts, follows)
- [x] crude internal did:plc generation
- [x] crude repo storage and manipulation
- [x] self-hosted did:web configuration
- [x] small-world registration and did:plc hosting configuration
- [ ] server-to-server interaction, at all (repo sync, etc)
- [ ] remote DID resolution (did:plc, did:web, etc)
- [ ] Lexicon schema validation of records/requests
- [ ] repo CAR file import: validation and app db update
- [ ] did:plc server interaction and chain validation
- [ ] complete app.bsky Lexicon implementation (notifications, likes, actors, etc)

Two main deployment configurations are supported:

- "fixed did:web accounts": accounts can only be registered at the CLI, and
  did:web is used instead of did:plc. A did:web DID document is served from
  `/.well-known/did.json` for matching domains, and a profile/feed is served
  from the homepage when domain matches
- "domain wildcard registration": accounts can be registered via XRPC, limited
  to handles under a specific hosting domain, and possibly requiring a secret
  invite code. did:plc identifiers are generated locally. web views are served
  from any domain, with registered handle domains being a profile/feed view

## Quickstart

TODO

- generate a PDS-wide secret key, and store as env configuration variable
- set other config variables (see `--help` or manpage)

## Deployment

TODO: rewrite this

- generate a PDS-wide secret key, and store as env configuration variable
- set other config variables (see `--help` or manpage)
- set up reverse proxy with SSL (eg, caddy or nginx or haproxy)
- run command from an appropriate local working directory (for databases)
- maybe set up a systemd unit file
- `adenosine-pds serve -vv`


## Implementation Details

This is a Rust programming language project.

Currently uses `rouille` as a (blocking) web framework; `rusqlite` for
(blocking) direct sqlite database interaction; `ipfs-sqlite-block-store` as a
(blocking) IPLD block store.

A bunch of cleanup and refactoring would be useful:

- cleaner ownership and referencing (reduce wasteful allocations)
- error handling (reduce `unwrap()` hacks)
- avoid mutex poisoning
- switch to async/await concurrency (replace or wrap blocking datastores) (eg,
  `axum`, `sqlx`, `iroh` blockstore)


### Vendored Code

The `iroh-car` crate, which has not (yet) been published to crates.io, has been
vendored out of the `iroh` project in to a sub-module of this crate.  This is
basically a verbatim copy, and the original copyright and license applies to
that code.

`ucan` support for the `p256` key type is implemented in this crate. Hoping to
get this upstreamed.


## Development and Contributions

Minimum Supported Rust Version (MSRV) is currently 1.61, using 2021 Rust
Edition.

Contributions, conversations, bug reports, and handwriten postcards are all
welcome. This is a low-effort hobby project so high-touch support, feature
requests, growth strategies, etc, probably will not be fielded.

The source code license is AGPLv3. Please acknowledge this in any initial
contributions, and also indicate whether and how you would like to be
acknowledged as a contributor (eg, a name and optionally a URL).
