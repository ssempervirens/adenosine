
The CAR format used for repository exports is compatible with IPLD. This means
the generic `ipfs` command-line tool (aka,
[kubo](https://docs.ipfs.tech/reference/kubo/cli/#ipfs-dag)) can be used to
import and explore the tree contents.

NOTE: The nested '/' on CID references in the `ipfs dag get` output below seems
to be a representation of the CBOR "tag" for CID types. There is not actually a
nested map/object with a key "/".  This is confirmed by inspecting the raw CBOR
using a python CBOR library (later on below).

    $ ipfs dag import example_repo.car 
    Pinned root     bafyreibqsl5vgfxmyngl6xdacp2mlc2j4u55jncocxdkcdwmq6xxtoqomq     success

    $ ipfs dag stat bafyreibqsl5vgfxmyngl6xdacp2mlc2j4u55jncocxdkcdwmq6xxtoqomq
    Size: 4430, NumBlocks: 36

    $ ipfs dag get bafyreibqsl5vgfxmyngl6xdacp2mlc2j4u55jncocxdkcdwmq6xxtoqomq | jq .
    {
      "root": {
        "/": "bafyreigxijbdrrybjetiaesyijvyheyfhvqak5iytpdsmpgmxgnwzqwpay"
      },
      "sig": {
        "/": {
          "bytes": "p+ZJTt63M0WFLy/KVeSmmG6kYuq2cvRbEBZPzHqeJe2IQfOkPl5G6x5P6ckggQ28eyc6q4FcKKHIjC8lS45pdA"
        }
      }
    }

    # /root
    $ ipfs dag get bafyreigxijbdrrybjetiaesyijvyheyfhvqak5iytpdsmpgmxgnwzqwpay | jq .
    {
      "auth_token": null,
      "data": {
        "/": "bafyreidrm6a2yucvumvug253af5fuxxwatgzid2wlq5c23kn5fvztwlbxi"
      },
      "meta": {
        "/": "bafyreib2pmfqcvbcpl6gtjgnaegvd4chn24cnat74pw3vrmfqr2jwd4n7m"
      },
      "prev": {
        "/": "bafyreidoekwhnzs2czs275nttwegy5j56foauwwjyr7qqfehyx5xnjctmy"
      }
    }

    # /root/meta
    $ ipfs dag get bafyreib2pmfqcvbcpl6gtjgnaegvd4chn24cnat74pw3vrmfqr2jwd4n7m | jq .
    {
      "datastore": "mst",
      "did": "did:plc:mj35frj7bgu6c2wszbtakt3e",
      "version": 1
    }

    # /root/data
    $ ipfs dag get bafyreidrm6a2yucvumvug253af5fuxxwatgzid2wlq5c23kn5fvztwlbxi | jq .
    {
      "e": [
        {
          "k": "app.bsky.follow/3jg4dmbn5y22a",
          "p": 0,
          "t": null,
          "v": {
            "/": "bafyreiegjavmgolbmo5yh54dy2ywkk2t3hg4ou7qugdggfdq4hedetddwm"
          }
        },
        {
          "k": "post/3jg4f3m6xas2a",
          "p": 9,
          "t": null,
          "v": {
            "/": "bafyreicaxix5adqs6e2yjmp4hui2ugfrn2ahquzrcdzb4b6j5l4xskansa"
          }
        },
        {
          "k": "com.example.thing/3jg4frpqngk2a",
          "p": 0,
          "t": null,
          "v": {
            "/": "bafyreieznv5quj54itdo7slq2rncjotmfxxtdnw3uwu7hi35euahp5a6im"
          }
        },
        {
          "k": "rk7a22a",
          "p": 24,
          "t": null,
          "v": {
            "/": "bafyreieznv5quj54itdo7slq2rncjotmfxxtdnw3uwu7hi35euahp5a6im"
          }
        }
      ],
      "l": null
    }

    # app.bsky.post/3jg4f3m6xas2a record
    $ ipfs dag get bafyreicaxix5adqs6e2yjmp4hui2ugfrn2ahquzrcdzb4b6j5l4xskansa
    {
    "text": "blah"
    }

Can also fetch raw MST nodes and records:

    # app.bsky.post/3jg4f3m6xas2a record
    $ ipfs block stat bafyreicaxix5adqs6e2yjmp4hui2ugfrn2ahquzrcdzb4b6j5l4xskansa
    Key: bafyreicaxix5adqs6e2yjmp4hui2ugfrn2ahquzrcdzb4b6j5l4xskansa
    Size: 11

    # app.bsky.post/3jg4f3m6xas2a record
    $ ipfs block get bafyreicaxix5adqs6e2yjmp4hui2ugfrn2ahquzrcdzb4b6j5l4xskansa > post.cbor

    # MST node
    $ ipfs block get bafyreidrm6a2yucvumvug253af5fuxxwatgzid2wlq5c23kn5fvztwlbxi > mst_node.cbor

Load CBOR records in python (`python3-cbor`):

    import cbor

    cbor.load(open("./post.cbor", "rb"))
    # {'text': 'blah'}

    cbor.load(open("./mst_node.cbor", "rb"))
    #{'e': [{'k': 'app.bsky.follow/3jg4dmbn5y22a',
    #   'p': 0,
    #   't': None,
    #   'v': Tag(42, b'\x00\x01q\x12 \x86H*\xc39ac\xbb\x83\xf7\x83\xc6\xb1e+S\xd9\xcd\xc7S\xf0\xa1\x86c\x14p\xe1\xc82Lc\xb3')},
    #  {'k': 'post/3jg4f3m6xas2a',
    #   'p': 9,
    #   't': None,
    #   'v': Tag(42, b'\x00\x01q\x12 @\xba/\xd0\x0e\x12\xf15\x84\xb1\xfc=\x11\xaa\x18\xb1n\x80xS1\x10\xf2\x1e\x07\xc9\xea\xf9y(\r\x90')},
    #  {'k': 'com.example.thing/3jg4frpqngk2a',
    #   'p': 0,
    #   't': None,
    #   'v': Tag(42, b"\x00\x01q\x12 \x99m{\n'\xbcD\xc6\xef\xc9p\xd4Z$\xbal-\xef1\xb6\xdb\xa5\xa9\xf3\xa3}%\x00w\xf4\x1eC")},
    #  {'k': 'rk7a22a',
    #   'p': 24,
    #   't': None,
    #   'v': Tag(42, b"\x00\x01q\x12 \x99m{\n'\xbcD\xc6\xef\xc9p\xd4Z$\xbal-\xef1\xb6\xdb\xa5\xa9\xf3\xa3}%\x00w\xf4\x1eC")}],
    # 'l': None}

Bigger set of records (showing more of MST structure):

    $ ipfs dag import bigger.cbor 
    Pinned root     bafyreifo6tgkutpfdub7s24whpjosgvh2wk2woezzs7udz23mt4exspzzi     success

    # /root/data
    $ ipfs dag get bafyreicou4t4riqlb3ipda27uhbcmdesh363ir6rsnozrk4aimqvblhh3y | jq .
    {
      "e": [
        {
          "k": "app.bsky.post/3jg6amq5bbs2a",
          "p": 0,
          "t": null,
          "v": {
            "/": "bafyreia4ggyqf23jyd3tspghzqlusqd7ob4hvjjfp6xkescosrc6telxdq"
          }
        },
        {
          "k": "niss2a",
          "p": 21,
          "t": {
            "/": "bafyreigmfcmuca3j3llcwtranptvx6ndygnrva3k3xixjyc7lqgmg7c3gu"
          },
          "v": {
            "/": "bafyreia4ggyqf23jyd3tspghzqlusqd7ob4hvjjfp6xkescosrc6telxdq"
          }
        },
        {
          "k": "w6ius2a",
          "p": 20,
          "t": {
            "/": "bafyreiatpb2iwp2ajsk6lqy5cqqdkny6lhj5dltwjq3tb7il3zpyyx23yu"
          },
          "v": {
            "/": "bafyreia4ggyqf23jyd3tspghzqlusqd7ob4hvjjfp6xkescosrc6telxdq"
          }
        }
      ],
      "l": {
        "/": "bafyreicg4wvrelokgneuylmckcp2ykxc67q4dqmcxjrfcmidjevn3qv53y"
      }
    }

    $ ipfs dag get bafyreigmfcmuca3j3llcwtranptvx6ndygnrva3k3xixjyc7lqgmg7c3gu | jq .
    {
      "e": [
        {
          "k": "app.bsky.post/3jg6amr5pek2a",
          "p": 0,
          "t": null,
          "v": {
            "/": "bafyreia4ggyqf23jyd3tspghzqlusqd7ob4hvjjfp6xkescosrc6telxdq"
          }
        },
        {
          "k": "md5c2a",
          "p": 21,
          "t": null,
          "v": {
            "/": "bafyreia4ggyqf23jyd3tspghzqlusqd7ob4hvjjfp6xkescosrc6telxdq"
          }
        },
        {
          "k": "s4zd22a",
          "p": 20,
          "t": null,
          "v": {
            "/": "bafyreia4ggyqf23jyd3tspghzqlusqd7ob4hvjjfp6xkescosrc6telxdq"
          }
        },
        {
          "k": "nxcs2a",
          "p": 21,
          "t": null,
          "v": {
            "/": "bafyreia4ggyqf23jyd3tspghzqlusqd7ob4hvjjfp6xkescosrc6telxdq"
          }
        },
        {
          "k": "t4ccc2a",
          "p": 20,
          "t": null,
          "v": {
            "/": "bafyreia4ggyqf23jyd3tspghzqlusqd7ob4hvjjfp6xkescosrc6telxdq"
          }
        },
        {
          "k": "n7cs2a",
          "p": 21,
          "t": null,
          "v": {
            "/": "bafyreia4ggyqf23jyd3tspghzqlusqd7ob4hvjjfp6xkescosrc6telxdq"
          }
        },
        {
          "k": "u3mas2a",
          "p": 20,
          "t": null,
          "v": {
            "/": "bafyreia4ggyqf23jyd3tspghzqlusqd7ob4hvjjfp6xkescosrc6telxdq"
          }
        },
        {
          "k": "mszs2a",
          "p": 21,
          "t": null,
          "v": {
            "/": "bafyreia4ggyqf23jyd3tspghzqlusqd7ob4hvjjfp6xkescosrc6telxdq"
          }
        },
        {
          "k": "v4rrk2a",
          "p": 20,
          "t": null,
          "v": {
            "/": "bafyreia4ggyqf23jyd3tspghzqlusqd7ob4hvjjfp6xkescosrc6telxdq"
          }
        },
        {
          "k": "n66s2a",
          "p": 21,
          "t": null,
          "v": {
            "/": "bafyreia4ggyqf23jyd3tspghzqlusqd7ob4hvjjfp6xkescosrc6telxdq"
          }
        }
      ],
      "l": null
    }

    $ ipfs dag get bafyreicg4wvrelokgneuylmckcp2ykxc67q4dqmcxjrfcmidjevn3qv53y | jq .
    {
    "e": [
        {
        "k": "app.bsky.post/3jg5zkr322c2a",
        "p": 0,
        "t": null,
        "v": {
            "/": "bafyreig2aqlsg4arslck64wbo2hnhe6k2a4z3z2sjfzh3uapv3a4zjld7e"
        }
        },
        {
        "k": "6amijiis2a",
        "p": 17,
        "t": null,
        "v": {
            "/": "bafyreia4ggyqf23jyd3tspghzqlusqd7ob4hvjjfp6xkescosrc6telxdq"
        }
        },
        {
        "k": "oy3tc2a",
        "p": 20,
        "t": null,
        "v": {
            "/": "bafyreia4ggyqf23jyd3tspghzqlusqd7ob4hvjjfp6xkescosrc6telxdq"
        }
        },
        {
        "k": "pkfrc2a",
        "p": 20,
        "t": null,
        "v": {
            "/": "bafyreia4ggyqf23jyd3tspghzqlusqd7ob4hvjjfp6xkescosrc6telxdq"
        }
        }
    ],
    "l": null
    }
