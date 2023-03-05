
Reminder of release process (new tagged versions):

final prep:
- consider running `cargo update` and checking that everything works
- `make lint` and fix anything
- `make fmt` and commit any fixes
- update CHANGELOG and increment version in `adenosine-*/Cargo.toml`. `make build` to
  update `Cargo.lock`, then commit
- `make test`, `make deb` and ensure build works and is clean
- ensure working directory is clean (no edits, even if unrelated to code changes)

push/tag/etc:
- when project is ready to share on crates.io, do: push to crates.io: `cargo publish -p adenosine-cli`, `cargo publish -p adenosine-pds`, `cargo publish -p adenosine`
    => add `--allow-dirty` if you have local "untracked" git files (and are confident!), and even `--no-verify` (if very confident!)
    => usually want to do this before git tag in case validation details come up
- create a git tag: `git tag vX.Y.Z -a -s -u bnewbold@robocracy.org -m "release X.Y.Z"`
- push git and tag: `git push`, `git push --tags`
- `cp ./target/release/adenosine adenosine_X.Y.Z_amd64_linux`
- `cp ./target/release/adenosine-pds adenosine-pds_X.Y.Z_amd64_linux`
- upload linux binary and deb package, eg: `ia upload adenosine-bin ./target/debian/*.deb adenosine*_amd64_linux`

homebrew / OS X (TODO):
- pull project
- build, then build package (?)
- upload package (?)
- update homebrew-adenosine repository
