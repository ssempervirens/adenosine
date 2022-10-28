
<div align="center">
<a href="https://en.wikipedia.org/wiki/File:ATP-xtal-3D-vdW.png">
<img src="extra/509px-ATP-xtal-3D-vdW.png" alt="Adenosine triphosphate molecule, from Wikipedia (CC-0 image by Ben Mills)">
</a>
</div>

`adenosine`: enthusiast-grade implementation of atproto.com in Rust
===================================================================

**Status:** it doesn't even work yet and will eat your data

This is a hobby project to implement components of the proposed Bluesky AT
Protocol ([atproto.com](https://atproto.com)) for federated social media, as
initially announced in Fall 2022. This might be interesting for other folks to
take a spin with, but isn't intended to host real content from real people. The
goal is to think through how the protocol might work by implementing it.

Three components planned:

- `adenosine-cli`: command-line client (`adenosine`), partially implemented and working
- `adenosine-pds`: "small world" personal data server implementation, with data in sqlite, not implemented
- `adenosine-tauri-gui`: minimal desktop GUI application, not implemented

Not currently planning on implementing the `did:plc` method. Would just stick
with `did:web` for now, either manually placing `/.well-known/did.json`
documents on existing servers or trying a wildcard DNS hack.


## Quickstart (CLI)

To work with an actual PDS server, you will need to get the
[`bluesky-social/atproto`](https://github.com/bluesky-social/atproto) prototype
PDS up and running locally. There is a separate
[quickstart](./notes/atproto_quickstart.md) on how to get that going.

If you run a Debian-base Linux distribution, you might be able to install an
existing package (TODO: link). Otherwise, you'll need to build the CLI tool
from source. Install a recent version of the Rust programming language
toolchain (eg, using [`rustup`](https://rustup.rs/)

The top level workplace includes the GUI and PDS code, which include huge
dependency trees and are very slow to build. If you just want the CLI, enter
the `adenosine-cli` directory before running any `cargo` commands.

To build, test, and locally install just the CLI (using `cargo`):

	cd adenosine-cli
	cargo install

To start interacting with a PDS, set the `ATP_HOST` environment variable. Then
either register a test account, or create a new session for an existing
account, and save the JWT token to the `ATP_AUTH_TOKEN`:

	# default port for bluesky-social/atproto implementation
	export ATP_HOST=http://localhost:2583

	# register a new account
	adenosine account register -u voltaire.test -p bogus -e voltaire@example.com
	{
	  "did": "did:plc:yqtuksvatmmgngd5nkkw75hn",
	  "jwt": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkaWQ6cGxjOnlxdHVrc3ZhdG1tZ25nZDVua2t3NzVobiIsImlhdCI6MTY2Njk5NjMwNn0.MMQa4JIQdwvhy-rjJ0kO-z8-KdoOL0Lto9JtOkK-lwE",
	  "username": "voltaire.test"
	}

	export ATP_AUTH_TOKEN=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkaWQ6cGxjOnlxdHVrc3ZhdG1tZ25nZDVua2t3NzVobiIsImlhdCI6MTY2Njk5NjMwNn0.MMQa4JIQdwvhy-rjJ0kO-z8-KdoOL0Lto9JtOkK-lwE

	# to clear the auth token env variable
	unset ATP_AUTH_TOKEN

	# create a new session (login) for existing account
	adenosine account login -u voltaire.test -p bogus
	{
	  "did": "did:plc:yqtuksvatmmgngd5nkkw75hn",
	  "jwt": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkaWQ6cGxjOnlxdHVrc3ZhdG1tZ25nZDVua2t3NzVobiIsImlhdCI6MTY2Njk5NjQxNX0.j2wcF1g9NxT_1AvYRiplNf_jtK6S81y3L38AkcBwOqY",
	  "name": "voltaire.test"
	}

	export ATP_AUTH_TOKEN=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkaWQ6cGxjOnlxdHVrc3ZhdG1tZ25nZDVua2t3NzVobiIsImlhdCI6MTY2Njk5NjQxNX0.j2wcF1g9NxT_1AvYRiplNf_jtK6S81y3L38AkcBwOqY

You could save the `ATP_HOST` and `ATP_AUTH_TOKEN` values in `~/.bashrc` so you
don't need to enter them every time.

Now you can start posting and poking around:

	adenosine bsky post "gruel again for breakfast"
	{
	  "cid": "bafyreig2aqlsg4arslck64wbo2hnhe6k2a4z3z2sjfzh3uapv3a4zjld7e",
	  "uri": "at://did:plc:yqtuksvatmmgngd5nkkw75hn/app.bsky.post/3jg5zkr322c2a"
	}


	adenosine ls at://voltaire.test
	app.bsky.post

There is a [manpage](./extra/adenosine.1.md) and [CLI-specific
README](./adenosine-cli/README.md) with more details and examples.


## Development

Mininum Supported Rust Version (MSRV) is currently 1.61, using 2021 Rust
Edition.

Contributions, conversations, bug reports, and handwriten postcards are all
welcome. As mentioned above this is a low-effort hobby project so high-touch
support, feature requests, growth strategies, etc, probably will not be
fielded.

The source code license is AGPLv3. Please acknowledge this in any initial
contributions, and also indicate whether and how you would like to be
acknowledged as a contributor (eg, a name and optionally a URL).
