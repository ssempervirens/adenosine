
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


## Example Commands

Generic atproto/XRPC commands:

    # configure host we are talking to
    export ATP_HOST=http://localhost:2583

    # register a new account
    adenosine account register -u voltaire.test -p bogus -e voltaire@example.com
    export ATP_AUTH_TOKEN=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkaWQ6cGxjOm1qMzVmcmo3Ymd1NmMyd3N6YnRha3QzZSIsImlhdCI6MTY2NjkzNzQxMn0.908LeimAXg1txMMH4k0_IcZAVJaFw1k7pVkScGMNcmE

    # or, login
    unset ATP_AUTH_TOKEN
    adenosine account login -u voltaire.test -p bogus
    export ATP_AUTH_TOKEN=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkaWQ6cGxjOm1qMzVmcmo3Ymd1NmMyd3N6YnRha3QzZSIsImlhdCI6MTY2NjkzNzcyMn0.7lXJ9xl6c-hrzUAR9YGLc4iFBn4nOJPbFX8TmYDHgdE

    # check config and setup
    adenosine status

    # not implemented server-side (yet?)
    adenosine account delete
    adenosine account logout

    # create, list, delete, update some records
    adenosine create com.example.thing a=123 b=cream

    adenosine ls at://hyde.test/app.bsky.post
    # TODO: bug in serve implementation? says "Could not find user"

    adenosine delete at://did:plc:mj35frj7bgu6c2wszbtakt3e/app.bsky.post/3jg4dqseut22a

    adenosine describe

    adenosine get at://hyde.test/app.bsky.post/asdf

    adenosine resolve voltaire.test

    adenosine repo root

    adenosine repo root did:plc:mj35frj7bgu6c2wszbtakt3e

    adenosine repo export did:plc:mj35frj7bgu6c2wszbtakt3e > example_repo.car

    adenosine xrpc get app.bsky.getHomeFeed


Example commands in bsky.app Lexicon:

    adenosine bsky feed

    adenosine bsky follow did:plc:mj35frj7bgu6c2wszbtakt3e

    adenosine bsky profile

    adenosine bsky profile voltaire.test

    adenosine get at://did:plc:mj35frj7bgu6c2wszbtakt3e/app.bsky.post/3jg4dqseut22a

