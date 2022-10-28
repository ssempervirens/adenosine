use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use regex::Regex;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DidOrHost {
    Did(String, String),
    Host(String),
}

impl FromStr for DidOrHost {
    type Err = anyhow::Error;

    /// DID syntax is specified in: <https://w3c.github.io/did-core/#did-syntax>
    ///
    /// Lazy partial hostname regex, isn't very correct.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref DID_RE: Regex =
                Regex::new(r"^did:([a-z]{1,64}):([a-zA-Z0-9\-.]{1,1024})$").unwrap();
        }
        lazy_static! {
            static ref HOSTNAME_RE: Regex =
                Regex::new(r"^[A-Za-z][A-Za-z0-9-]*(\.[A-Za-z][A-Za-z0-9-]*)+$").unwrap();
        }
        if let Some(caps) = DID_RE.captures(s) {
            Ok(Self::Did(caps[1].to_string(), caps[2].to_string()))
        } else if HOSTNAME_RE.is_match(s) {
            Ok(Self::Host(s.to_string()))
        } else {
            Err(anyhow!("does not match as a DID or hostname: {}", s))
        }
    }
}

impl fmt::Display for DidOrHost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Host(v) => write!(f, "{}", v),
            Self::Did(m, v) => write!(f, "did:{}:{}", m, v),
        }
    }
}

#[test]
fn test_didorhost() {
    assert_eq!(
        DidOrHost::from_str("hyde.test").unwrap(),
        DidOrHost::Host("hyde.test".to_string())
    );
    assert_eq!(
        DidOrHost::from_str("did:method:blah").unwrap(),
        DidOrHost::Did("method".to_string(), "blah".to_string())
    );

    assert!(DidOrHost::from_str("barestring").is_err());
    assert!(DidOrHost::from_str("did:partial:").is_err());
    assert!(DidOrHost::from_str("").is_err());
    assert!(DidOrHost::from_str(" ").is_err());
    assert!(DidOrHost::from_str("1234").is_err());

    assert!(DidOrHost::from_str("mutli.part.domain").is_ok());
    assert!(DidOrHost::from_str("did:is:weird").is_ok());
    assert!(DidOrHost::from_str("did:plc:bv6ggog3tya2z3vxsub7hnal").is_ok());
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AtUri {
    pub repository: DidOrHost,
    pub collection: Option<String>,
    pub record: Option<String>,
    pub fragment: Option<String>,
}

impl FromStr for AtUri {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref ATURI_RE: Regex = Regex::new(r"^at://([a-zA-Z0-9:_\.-]+)(/([a-zA-Z0-9\.]+))?(/([a-zA-Z0-9\.-]+))?(#([a-zA-Z0-9/-]+))?$").unwrap();
        }
        if let Some(caps) = ATURI_RE.captures(s) {
            let uri = AtUri {
                repository: DidOrHost::from_str(&caps[1])?,
                collection: caps.get(3).map(|v| v.as_str().to_string()),
                record: caps.get(5).map(|v| v.as_str().to_string()),
                fragment: caps.get(7).map(|v| v.as_str().to_string()),
            };
            Ok(uri)
        } else {
            Err(anyhow!("couldn't parse as an at:// URI: {}", s))
        }
    }
}

impl fmt::Display for AtUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at://{}", self.repository)?;
        if let Some(ref c) = self.collection {
            write!(f, "/{}", c)?;
        };
        if let Some(ref r) = self.record {
            write!(f, "/{}", r)?;
        };
        if let Some(ref v) = self.fragment {
            write!(f, "#{}", v)?;
        };
        Ok(())
    }
}

#[test]
fn test_aturi() {
    assert!(AtUri::from_str("at://bob.com").is_ok());
    assert!(AtUri::from_str("at://did:plc:bv6ggog3tya2z3vxsub7hnal").is_ok());
    assert!(AtUri::from_str("at://bob.com/io.example.song").is_ok());
    assert!(AtUri::from_str("at://bob.com/io.example.song/3yI5-c1z-cc2p-1a").is_ok());
    assert!(AtUri::from_str("at://bob.com/io.example.song/3yI5-c1z-cc2p-1a#/title").is_ok());
    assert!(
        AtUri::from_str("at://did:plc:ltk4reuh7rkoy2frnueetpb5/app.bsky.follow/3jg23pbmlhc2a")
            .is_ok()
    );

    let uri = AtUri {
        repository: DidOrHost::Did("some".to_string(), "thing".to_string()),
        collection: Some("com.atproto.record".to_string()),
        record: Some("asdf-123".to_string()),
        fragment: Some("/path".to_string()),
    };
    assert_eq!(
        "at://did:some:thing/com.atproto.record/asdf-123#/path",
        uri.to_string()
    );
    println!("{:?}", AtUri::from_str(&uri.to_string()));
    assert!(AtUri::from_str(&uri.to_string()).is_ok());

    let uri = AtUri::from_str("at://bob.com/io.example.song/3yI5-c1z-cc2p-1a#/title").unwrap();
    assert_eq!(uri.repository, DidOrHost::Host("bob.com".to_string()));
    assert_eq!(uri.collection, Some("io.example.song".to_string()));
    assert_eq!(uri.record, Some("3yI5-c1z-cc2p-1a".to_string()));
    assert_eq!(uri.fragment, Some("/title".to_string()));
}
