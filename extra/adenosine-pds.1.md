NAME
====

adenosine-pds: small-world atproto.com Personal Data Server

SYNOPSIS
========

adenosine-pds \[OPTIONS\] \<COMMAND\> \<ARGS\>

DESCRIPTION
===========

This is a simple, enthusiast-grade AT Protocol (atproto.com) personal
data server (\"PDS\") implementation. It targets \"small-world\" uses
cases of the protocol, for example personal or organizational
self-hosting.

This is a work-in-progress, is not spec-compliant, will not be
backwards/forwards compatible, and does not have an upgrade/migration
path.

COMMANDS
========

**generate-secret**

> Creates a new random secret key for PDS use

**serve** \[OPTIONS\]

> Runs the server. See options below

**import** \<car-path\> \[\--alias \<alias\>\]

> Loads a CAR file into the repository blockstore

**inspect**

> Prints information about repositories in the blockstore (likely to
> deprecate)

OPTIONS
=======

**-h, \--help**

> Prints help information

**-V, \--version**

> Prints version information

**-v, \--verbose**

> Pass many times for more log output By default, it\'ll only report
> errors. Passing \`-v\` one time also prints warnings, \`-vv\` enables
> info logging, \`-vvv\` debug, and \`-vvvv\` trace.

**\--atp-db \<path\>** \[env: ATP\_ATP\_DB\]

> File path of sqlite database holding system/application data

**\--block-db \<path\>** \[env: ATP\_BLOCK\_DB\]

> File path of sqlite database holding repository data (blockstore)

SERVE OPTIONS
-------------

**\--homepage-handle \<homepage-handle\>** \[env:
ATP\_PDS\_HOMEPAGE\_HANDLE\]

> Optionally, override domain name check and force the homepage to
> display the account page for this handle

**\--invite-code \<invite-code\>** \[env: ATP\_PDS\_INVITE\_CODE\]

> Optionally, require an invite code to sign up. This is just a single
> secret value

**\--pds-secret-key \<pds-secret-key\>** \[env: ATP\_PDS\_SECRET\_KEY\]

> Secret key, encoded in hex. Use \'generate-secret\' to create a new
> one

**\--port \<port\>** \[env: ATP\_PDS\_PORT\] \[default: 3030\]

> Localhost port to listen on

**\--public-url \<public-url\>** \[env: ATP\_PDS\_PUBLIC\_URL\]

> A \"public URL\" for the PDS gets embedded in DID documents. If one is
> not provided, a localhost value will be used, which will not actually
> work for inter-PDS communication

**\--registration-domain \<registration-domain\>** \[env:
ATP\_PDS\_REGISTRATION\_DOMAIN\]

> If provided, allow registration for the given base domain name

GETTING STARTED
===============

TODO
