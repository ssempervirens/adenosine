
`adenosine-cli`: command-line client for AT protocol (atproto.com)
==================================================================

This is a simple, enthusiast-grade CLI client for the work-in-progress AT
Protocol ([atproto.com](https://atproto.com)). Sort of like the `http` command
([HTTPie](https://httpie.io/)).

It is an entirely "delegated" client, which means that it does not store or
cache any user content locally; everything works by making requests to a
Personal Data Server (PDS), which is usually a remote service.

The only real utility of this tool currently is messing around with prototype
implementations, possibly while developing them.

This client does not currently do any schema validation of Lexicons, either at
compile time or runtime (eg, dynamically fetching Lexicons). The Bluesky
Lexicon (bsky.app) is partially supported with helper commands.

There is a [manpage](../extra/adenosine.1.md) with usage details and examples.

Features:

- [x] generic XRPC invocation (GET and POST)
- [x] com.atproto Lexicon implementation
- [x] repo CAR file import/export
- [x] partial app.bsky Lexicon implementation
- [ ] complete app.bsky Lexicon implementation
- [ ] did:plc server interaction and chain validation
- [ ] pretty printing of bsky records (eg, timeline) with JSON as a flag
- [ ] test coverage

Possible future features:

- [ ] Lexicon schema validation of records/requests
- [ ] DID resolution to repo location (independent of PDS)
- [ ] save login/configuration in homedir dotfile

## Installation

`adenosine-cli` is implemented in Rust, and in theory could be distributed in
binary form for multiple platforms.

TODO: debian packages for linux

Otherwise, you'll need to build the CLI tool from source. Install a recent
version of the Rust programming language toolchain (eg, using
[`rustup`](https://rustup.rs/).

The top level workplace includes the PDS code, which has a huge dependency tree
and is slow to build. If you just want the CLI, enter the `adenosine-cli`
directory before running any `cargo` commands:

	cd adenosine-cli
	cargo install


## Quickstart

You'll need an actual PDS server to interact with. As of November 2022, this
could be an independent implementation like `adenosine-pds`, or the more
official official [`bluesky-social/atproto`](https://github.com/bluesky-social/atproto)
prototype (see [local atproto dev quickstart](./../notes/atproto_quickstart.md)).


Set the `ATP_PDS_HOST` environment variable. Then either register a test account,
or create a new session for an existing account, and save the JWT token to the
`ATP_AUTH_TOKEN`:

	# default port for bluesky-social/atproto implementation
	export ATP_PDS_HOST=http://localhost:2583

	# default port for adenosine-pds implementation
	export ATP_PDS_HOST=http://localhost:3030

	# register a new account
	adenosine account register -u voltaire.test -p bogus -e voltaire@example.com
	{
	  "did": "did:plc:yqtuksvatmmgngd5nkkw75hn",
	  "jwt": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkaWQ6cGxjOnlxdHVrc3ZhdG1tZ25nZDVua2t3NzVobiIsImlhdCI6MTY2Njk5NjMwNn0.MMQa4JIQdwvhy-rjJ0kO-z8-KdoOL0Lto9JtOkK-lwE",
	  "username": "voltaire.test"
	}

	export ATP_AUTH_TOKEN=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkaWQ6cGxjOnlxdHVrc3ZhdG1tZ25nZDVua2t3NzVobiIsImlhdCI6MTY2Njk5NjMwNn0.MMQa4JIQdwvhy-rjJ0kO-z8-KdoOL0Lto9JtOkK-lwE

	# to clear the auth token env variable in current shell
	unset ATP_AUTH_TOKEN

	# create a new session (login) for existing account
	adenosine account login -u voltaire.test -p bogus
	{
	  "did": "did:plc:yqtuksvatmmgngd5nkkw75hn",
	  "jwt": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkaWQ6cGxjOnlxdHVrc3ZhdG1tZ25nZDVua2t3NzVobiIsImlhdCI6MTY2Njk5NjQxNX0.j2wcF1g9NxT_1AvYRiplNf_jtK6S81y3L38AkcBwOqY",
	  "name": "voltaire.test"
	}

	export ATP_AUTH_TOKEN=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkaWQ6cGxjOnlxdHVrc3ZhdG1tZ25nZDVua2t3NzVobiIsImlhdCI6MTY2Njk5NjQxNX0.j2wcF1g9NxT_1AvYRiplNf_jtK6S81y3L38AkcBwOqY

You could save the `ATP_PDS_HOST` and `ATP_AUTH_TOKEN` values in `~/.bashrc` so you
don't need to enter them every time.

Now you can start posting and poking around:

    adenosine status

	adenosine bsky post "gruel again for breakfast"
	{
	  "cid": "bafyreig2aqlsg4arslck64wbo2hnhe6k2a4z3z2sjfzh3uapv3a4zjld7e",
	  "uri": "at://did:plc:yqtuksvatmmgngd5nkkw75hn/app.bsky.post/3jg5zkr322c2a"
	}

	adenosine ls at://voltaire.test


## Development and Contributions

Minimum Supported Rust Version (MSRV) is currently 1.61, using 2021 Rust
Edition.

Contributions, conversations, bug reports, and handwriten postcards are all
welcome. This is a low-effort hobby project so high-touch support, feature
requests, growth strategies, etc, probably will not be fielded.

The source code license is AGPLv3. Please acknowledge this in any initial
contributions, and also indicate whether and how you would like to be
acknowledged as a contributor (eg, a name and optionally a URL).
