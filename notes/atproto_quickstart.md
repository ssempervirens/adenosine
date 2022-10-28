
ATP (atproto.com) Development Quickstart
========================================

Setup javascript development environment, assuming you have a checkout of the atproto repo `$SOMEWHERE`:

- on linux/macos, install `nvm` for your user to manage a fresh/ephemeral
  nodejs environment, separate from any system-managed nodejs installation
- `nvm install 18`
- `nvm use 18`
- `npm install --global yarn`
- `cd $SOMEWHERE`
- `yarn install --frozen-lockfile`
- `yarn build`

To enter the dev environment in a new terminal:

- `nvm use 18`
- `cd $SOMEWHERE`

Run an atproto example PDS server (in memory), and create a test user:

- (enter dev environment)
- `cd packages/dev_env`
- `yarn run start`
- `> status()`
- `> mkuser("hyde")`
- `> user("hyde.test")`

You should be able to fetch, eg, <http://localhost:2583/xrpc/com.atproto.getAccountsConfig> (depending on local port).

The CLI doesn't work for me, but if it was working you would do something like:

- (enter dev environment)
- `cd packages/cli`
- `yarn run cli --help`

## HTTPie CLI Interaction

Instead of the CLI, can use the `http` command (aka, HTTPie CLI). First do something like `export HOST=http://localhost:2583`

    http get $HOST/xrpc/com.atproto.getAccountsConfig
    => {
        "availableUserDomains": [
            "test"
        ],
        "inviteCodeRequired": false
    }

### Accounts/Sessions

    http post $HOST/xrpc/com.atproto.createAccount email=bogus@example.com username=jekyll password=bogus
    => {
        "did": "did:plc:uek4uxvrpht7ymim5nhopdvn",
        "jwt": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkaWQ6cGxjOnVlazR1eHZycGh0N3ltaW01bmhvcGR2biIsImlhdCI6MTY2NjgyNzQ2NH0.788_uqKmglir-J-Oexsawp83Kn3g-J62kZ5ITgrGu5s",
        "username": "jekyll"
    }

    http post $HOST/xrpc/com.atproto.createAccount email=bogus2@example.com username=jekyll.test password=bogus
    => {
        "did": "did:plc:lewcwgmmp7rfi4z3bs3ls5tp",
        "jwt": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkaWQ6cGxjOmxld2N3Z21tcDdyZmk0ejNiczNsczV0cCIsImlhdCI6MTY2NjgyNzczNn0.S2IaqAS1bx6XzLpz7WybwGbUCsex4zPlMNm1sRKyzh0",
        "username": "jekyll.test"
    }

    http post $HOST/xrpc/com.atproto.createSession username=jekyll.test password=bogus
    => {
        "did": "did:plc:lewcwgmmp7rfi4z3bs3ls5tp",
        "jwt": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkaWQ6cGxjOmxld2N3Z21tcDdyZmk0ejNiczNsczV0cCIsImlhdCI6MTY2NjgyODIwMH0.Bo-dkrKROYoUlRmrFy8_c1AF7lGK17g1JyRnJUHROyQ",
        "name": "jekyll.test"
    }

    http get $HOST/xrpc/com.atproto.resolveName name==hyde.test
    => { "did": "did:plc:ex3si527cd2aogbvidkooozc" }

    http get $HOST/xrpc/com.atproto.resolveName name==jekyll
    http get $HOST/xrpc/com.atproto.resolveName name==at://pfrazee.com
    => { "message": "Unable to resolve name" }

    http get $HOST/xrpc/com.atproto.resolveName name==jekyll.test
    => { "did": "did:plc:lewcwgmmp7rfi4z3bs3ls5tp" }

    http get $HOST/xrpc/com.atproto.repoDescribe user==jekyll.test
    => { ... }

## Create bsky.app Records

Try doing some bsky.app stuff. This requires auth, using JWT as a bearer:

    export TOKEN=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkaWQ6cGxjOmxld2N3Z21tcDdyZmk0ejNiczNsczV0cCIsImlhdCI6MTY2NjgyODIwMH0.Bo-dkrKROYoUlRmrFy8_c1AF7lGK17g1JyRnJUHROyQ

    http get $HOST/xrpc/app.bsky.getAuthorFeed Authorization:"Bearer $TOKEN" author==jekyll.test
    => { "feed": [] }

    http get $HOST/xrpc/app.bsky.getAuthorFeed Authorization:"Bearer $TOKEN" author==jekyll.test

    http get $HOST/xrpc/app.bsky.getHomeFeed Authorization:"Bearer $TOKEN"

    http get $HOST/xrpc/app.bsky.getProfile Authorization:"Bearer $TOKEN" user==jekyll.test
    => {
        "did": "did:plc:lewcwgmmp7rfi4z3bs3ls5tp",
        "followersCount": 0,
        "followsCount": 0,
        "myState": {},
        "name": "jekyll.test",
        "pinnedBadges": [],
        "postsCount": 0
    }

    http post $HOST/xrpc/app.bsky.updateProfile Authorization:"Bearer $TOKEN" displayName="Mr. Jekyll" description="You know me"
    => { "message": "could not parse current profile" }
    # (seems like a bug)

We create bsky.app records using generic atproto.com requests:

    # note distinction between "=" and "==" in HTTPie args
    http post $HOST/xrpc/com.atproto.repoCreateRecord Authorization:"Bearer $TOKEN" did==did:plc:lewcwgmmp7rfi4z3bs3ls5tp collection==app.bsky.post text="this is my first post"
    => {
        "cid": "bafyreidyxly4tjc2vyorvwdppc5oqiz6e5inosea3mnvge5fjnuzot7efu",
        "uri": "at://did:plc:lewcwgmmp7rfi4z3bs3ls5tp/app.bsky.post/3jfz5q2xx2c2a"
    }


    http post $HOST/xrpc/com.atproto.repoCreateRecord Authorization:"Bearer $TOKEN" did==did:plc:lewcwgmmp7rfi4z3bs3ls5tp collection==app.bsky.post text="this is a second post, yadda yadda"
    => { ... }

Try some inter-user stuff:

    http post $HOST/xrpc/com.atproto.createSession username=hyde.test password=hyde-pass
    => {
        "did": "did:plc:ex3si527cd2aogbvidkooozc",
        "jwt": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkaWQ6cGxjOmV4M3NpNTI3Y2QyYW9nYnZpZGtvb296YyIsImlhdCI6MTY2NjgyOTM5M30.UvZgTqvaJICONa1wIUT1bny7u3hqVAqWhWy3qeuyZrE",
        "name": "hyde.test"
    }

    export HYDETOKEN=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkaWQ6cGxjOmV4M3NpNTI3Y2QyYW9nYnZpZGtvb296YyIsImlhdCI6MTY2NjgyOTM5M30.UvZgTqvaJICONa1wIUT1bny7u3hqVAqWhWy3qeuyZrE

    http post $HOST/xrpc/com.atproto.repoCreateRecord Authorization:"Bearer $HYDETOKEN" did==did:plc:ex3si527cd2aogbvidkooozc collection==app.bsky.post subject=at://did:plc:lewcwgmmp7rfi4z3bs3ls5tp
    => {
        "cid": "bafyreihncbwcmnyngqigdglyzhur2zo53gfnhljoz32gnhqrnd3tgjelim",
        "uri": "at://did:plc:ex3si527cd2aogbvidkooozc/app.bsky.post/3jfz5z27hxc2a"
    }

    http post $HOST/xrpc/com.atproto.repoCreateRecord Authorization:"Bearer $HYDETOKEN" did==did:plc:ex3si527cd2aogbvidkooozc collection==app.bsky.like subject=at://did:plc:lewcwgmmp7rfi4z3bs3ls5tp/app.bsky.post/3jfz5q2xx2c2a
    => {
        "cid": "bafyreigjrx6l4dpmwxm6fow3k4ton3qxohr6d2yuf2c77umjyfnscwj3ii",
        "uri": "at://did:plc:ex3si527cd2aogbvidkooozc/app.bsky.like/3jfz62eoytk2a"
    }

    http post $HOST/xrpc/com.atproto.repoCreateRecord Authorization:"Bearer $TOKEN" did==did:plc:lewcwgmmp7rfi4z3bs3ls5tp collection==app.bsky.post text="this is a third post, yadda yadda"
    => { ... }

    http get $HOST/xrpc/app.bsky.getHomeFeed Authorization:"Bearer $HYDETOKEN"
    => { "feed": [] }
    # not working yet? or I may just be confused


    http get $HOST/xrpc/com.atproto.repoDescribe user==jekyll.test
    # NOTE: 'collections' contains duplicate entries

    http get $HOST/xrpc/com.atproto.repoDescribe user==hyde.test

### Repo Sync (binary CAR file export)

Try fetching raw repo content, and parse start of CAR result using `python3-cbor` package:

    http get $HOST/xrpc/com.atproto.syncGetRoot did==did:plc:ex3si527cd2aogbvidkooozc
    => { "root": "bafyreigpahlcbvvhriogrzdwwpufjqkd4ybsjzeztnzttqulkszrrjo5nq" }

    http get $HOST/xrpc/com.atproto.syncGetRepo did==did:plc:ex3si527cd2aogbvidkooozc >> example_repo.cbor

    python3
    >>> import cbor
    >>> raw = open("./example_repo.cbor").read()
    >>> raw[0]
    58
    >>> cbor.loads(raw[1:])
    {'roots': [Tag(42, b'\x00\x01q\x12 \xcf\x01\xd6 \xd6\xa7\x8a\x1ch\xe4v\xb3\xe8T\xc1C\xe6\x03$\xe4\x99\x9bs9\xc2\x8bT\xb3\x18\xa5\xddl')], 'version': 1}
    
 See also <https://ipld.io/specs/transport/car/carv1>
