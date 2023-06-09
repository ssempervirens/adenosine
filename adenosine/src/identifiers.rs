use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use regex::Regex;
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;
use std::time::SystemTime;

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
            Self::Host(v) => write!(f, "{v}"),
            Self::Did(m, v) => write!(f, "did:{m}:{v}"),
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

    assert!(DidOrHost::from_str("multi.part.domain").is_ok());
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
            static ref ATURI_RE: Regex = Regex::new(r"^at://([a-zA-Z0-9:_\.-]+)(/(([a-zA-Z0-9\.]+))?)?(/(([a-zA-Z0-9\.-]+))?)?(#([a-zA-Z0-9/-]+))?$").unwrap();
        }
        if let Some(caps) = ATURI_RE.captures(s) {
            let uri = AtUri {
                repository: DidOrHost::from_str(&caps[1])?,
                collection: caps.get(4).map(|v| v.as_str().to_string()),
                record: caps.get(7).map(|v| v.as_str().to_string()),
                fragment: caps.get(9).map(|v| v.as_str().to_string()),
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
            write!(f, "/{c}")?;
        };
        if let Some(ref r) = self.record {
            write!(f, "/{r}")?;
        };
        if let Some(ref v) = self.fragment {
            write!(f, "#{v}")?;
        };
        Ok(())
    }
}

#[test]
fn test_aturi() {
    assert!(AtUri::from_str("at://bob.com").is_ok());
    assert!(AtUri::from_str("at://bob.com/").is_ok());
    assert!(AtUri::from_str("at://did:plc:bv6ggog3tya2z3vxsub7hnal").is_ok());
    assert!(AtUri::from_str("at://bob.com/io.example.song").is_ok());
    assert!(AtUri::from_str("at://bob.com/io.example.song/").is_ok());
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

    let uri = AtUri::from_str("at://bob.com/io.example.song/").unwrap();
    assert_eq!(uri.repository, DidOrHost::Host("bob.com".to_string()));
    assert_eq!(uri.collection, Some("io.example.song".to_string()));
}

/// A String (newtype) representing an NSID
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct Nsid(String);

impl FromStr for Nsid {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref NSID_RE: Regex = Regex::new(r"^([a-z][a-z0-9-]+\.)+[a-zA-Z0-9-]+$").unwrap();
        }
        if NSID_RE.is_match(s) {
            Ok(Self(s.to_string()))
        } else {
            Err(anyhow!("does not match as an NSID: {}", s))
        }
    }
}

impl Deref for Nsid {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Nsid {
    pub fn domain(&self) -> String {
        self.rsplit('.').skip(1).collect::<Vec<&str>>().join(".")
    }

    pub fn name(&self) -> String {
        self.split('.')
            .last()
            .expect("multiple segments in NSID")
            .to_string()
    }
}

impl fmt::Display for Nsid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[test]
fn test_nsid() {
    assert!(Nsid::from_str("com.atproto.recordType").is_ok());

    let nsid = Nsid::from_str("com.atproto.recordType").unwrap();
    assert_eq!(nsid.domain(), "atproto.com".to_string());
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct Did(String);

impl FromStr for Did {
    type Err = anyhow::Error;

    /// DID syntax is specified in: <https://w3c.github.io/did-core/#did-syntax>
    ///
    /// This regex does not follow that definition exactly.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref DID_RE: Regex =
                Regex::new(r"^did:([a-z]{1,32}):([a-zA-Z0-9\-.]{1,256})$").unwrap();
        }
        if DID_RE.is_match(s) {
            Ok(Self(s.to_string()))
        } else {
            Err(anyhow!("does not match as a DID: {}", s))
        }
    }
}

impl Deref for Did {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Did {
    pub fn did_type(&self) -> String {
        self.split(':').nth(1).unwrap().to_string()
    }
}

impl fmt::Display for Did {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[test]
fn test_did() {
    assert!(Did::from_str("did:web:asdf.org").is_ok());
    assert!(Did::from_str("did:plc:asdf").is_ok());

    assert!(Did::from_str("bob.com").is_err());
    assert!(Did::from_str("").is_err());
    assert!(Did::from_str("did:").is_err());
    assert!(Did::from_str("did:plc:").is_err());
    assert!(Did::from_str("plc:asdf").is_err());
    assert!(Did::from_str("DID:thing:thang").is_err());

    assert_eq!(
        Did::from_str("did:web:asdf.org").unwrap().did_type(),
        "web".to_string()
    );
}

lazy_static! {
    /// Sortable base32 encoding, as bluesky implements/defines
    static ref BASE32SORT: data_encoding::Encoding = {
        let mut spec = data_encoding::Specification::new();
        spec.symbols.push_str("234567abcdefghijklmnopqrstuvwxyz");
        spec.padding = None;
        spec.encoding().unwrap()
    };
}

/// A string identifier for individual records, based on UNIX timestamp in microseconds.
///
/// See also: https://github.com/bluesky-social/atproto/issues/334
///
/// Pretty permissive about what can be parsed/accepted, because there were some old TIDs floating
/// around with weird format.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tid(String);

impl FromStr for Tid {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref TID_RE: Regex = Regex::new(r"^[0-9a-zA-Z-]{13,20}$").unwrap();
        }
        if TID_RE.is_match(s) {
            Ok(Self(s.to_string()))
        } else {
            Err(anyhow!("does not match as a TID: {}", s))
        }
    }
}

impl Deref for Tid {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for Tid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Tid {
    pub fn new(micros: u64, clock_id: u16) -> Self {
        // 53 bits of millis
        let micros = micros & 0x001FFFFFFFFFFFFF;
        // 10 bits of clock ID
        let clock_id = clock_id & 0x03FF;
        let val: u64 = (micros << 10) | (clock_id as u64);
        // big-endian encoding
        let enc = BASE32SORT.encode(&val.to_be_bytes());
        Tid(format!(
            "{}-{}-{}-{}",
            &enc[0..4],
            &enc[4..7],
            &enc[7..11],
            &enc[11..13]
        ))
    }
}

#[test]
fn test_tid() {
    Tid::from_str("3yI5-c1z-cc2p-1a").unwrap();
    assert!(Tid::from_str("3jg6anbimrc2a").is_ok());
    assert!(Tid::from_str("3yI5-c1z-cc2p-1a").is_ok());

    Tid::from_str("asdf234as4asdf234").unwrap();
    assert!(Tid::from_str("asdf234as4asdf234").is_ok());

    assert!(Tid::from_str("").is_err());
    assert!(Tid::from_str("com").is_err());
    assert!(Tid::from_str("com.blah.Thing").is_err());
    assert!(Tid::from_str("did:stuff:blah").is_err());

    let t1 = Tid::new(0, 0);
    assert_eq!(t1.to_string(), "2222-222-2222-22".to_string());
}

/// TID Generator
///
/// This version uses 53-bit microsecond counter (since UNIX epoch), and a random 10-bit clock id.
///
/// If the current timestamp is not greater than the last timestamp (either because clock did not
/// advance monotonically, or multiple TIDs were generated in the same microsecond (very unlikely),
/// the timestamp is simply incremented.
pub struct Ticker {
    last_timestamp: u64,
    clock_id: u16,
}

impl Ticker {
    pub fn new() -> Self {
        let mut ticker = Self {
            last_timestamp: 0,
            // mask to 10 bits
            clock_id: rand::random::<u16>() & 0x03FF,
        };
        // prime the pump
        ticker.next_tid();
        ticker
    }

    pub fn next_tid(&mut self) -> Tid {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("timestamp in micros since UNIX epoch")
            .as_micros() as u64;
        // mask to 53 bits
        let now = now & 0x001FFFFFFFFFFFFF;
        if now > self.last_timestamp {
            self.last_timestamp = now;
        } else {
            self.last_timestamp += 1;
        }
        Tid::new(self.last_timestamp, self.clock_id)
    }
}

impl Default for Ticker {
    fn default() -> Self {
        Self::new()
    }
}

#[test]
fn test_ticker() {
    let mut ticker = Ticker::new();
    let mut prev = ticker.next_tid();
    let mut next = ticker.next_tid();
    for _ in [0..100] {
        println!("{next} >? {prev}");
        assert!(next > prev);
        prev = next;
        next = ticker.next_tid();
    }
    println!("{prev}");
    assert_eq!(prev, Tid::from_str(&prev).unwrap());
    assert_eq!(next[13..16], prev[13..16]);

    let mut other_ticker = Ticker::new();
    let other = other_ticker.next_tid();
    assert!(other > next);
    assert!(next[13..16] != other[13..16]);
}
