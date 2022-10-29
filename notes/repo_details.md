
It seems like the current MST format does not match the current atproto docs
(https://atproto.com/specs/atp), and some pieces are missing.

## Repo Questions

**What CID codec and hash is used?**

For the IPLD nodes and MST tree, it looks like base32, dag-cbor, sha2-256, with
256 bits of hash.

**What cryptography is used for signatures?**

Haven't dug through code yet to figure it out.

**What is the auth_token (JWT) for?**

Not sure, but it is often null in the current implementation. Maybe has to do
with clients that don't have the full signing key?

## MST Questions

**How to determine "layer" of tree for a given key? AKA, how to count "leading zeros"?**

- take hash of the full key (UTF-8 string)
- encode in baseN format, depending on "fanout" configuration
- the "zero char" is the char which encoding '0x00' (single zero byte) in the baseN codec, and taking the first char returned. this is usually (always?) '0'
- for a given hashed+encoded key, count how many leading chars match the "zero char"

**What is the prefix key compression relative to ("previous node")?**

Based on output from the current implementation, it seems local to individual
MST nodes. AKA, each MST node has enough information to recover all the keys
for all the records indicated in that node.
