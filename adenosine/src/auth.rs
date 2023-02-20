use anyhow::anyhow;
pub use anyhow::Result;
use serde_json::Value;

/// Tries to parse a DID internal identifier from a JWT (as base64-encoded token)
pub fn parse_did_from_jwt(jwt: &str) -> Result<String> {
    let second_b64 = jwt.split('.').nth(1).ok_or(anyhow!("couldn't parse JWT"))?;
    let second_json: Vec<u8> = base64::decode_config(second_b64, base64::URL_SAFE)?;
    let obj: Value = serde_json::from_slice(&second_json)?;
    // trying to also support pulling "aud" as DID; not sure this is actually correct use of
    // UCAN/JWT semantics?
    let did = obj["sub"]
        .as_str()
        .or(obj["aud"].as_str())
        .ok_or(anyhow!("couldn't find DID subject in JWT"))?
        .to_string();
    if !did.starts_with("did:") {
        return Err(anyhow!("couldn't find DID subject in JWT"));
    }
    Ok(did)
}

#[test]
fn test_parse_jwt() {
    assert!(parse_did_from_jwt(".").is_err());
    // JWT from atproto ("sub")
    assert_eq!(
        parse_did_from_jwt("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkaWQ6cGxjOmV4M3NpNTI3Y2QyYW9nYnZpZGtvb296YyIsImlhdCI6MTY2NjgyOTM5M30.UvZgTqvaJICONa1wIUT1bny7u3hqVAqWhWy3qeuyZrE").unwrap(),
        "did:plc:ex3si527cd2aogbvidkooozc",
    );
    // UCAN from adenosine-pds ("aud")
    assert_eq!(
        parse_did_from_jwt("eyJhbGciOiJFUzI1NiIsInR5cCI6IkpXVCIsInVjdiI6IjAuOS4wLWNhbmFyeSJ9.eyJhdHQiOltdLCJhdWQiOiJkaWQ6cGxjOnM3b25ieWphN2MzeXJzZ3Zob2xrbHM1YiIsImV4cCI6MTY3NTM4Mzg2NywiZmN0IjpbXSwiaXNzIjoiZGlkOmtleTp6RG5hZWRHVGJkb0Frb1NlOG96a3k1WHAzMjZTVFpUSm50aDlHY2dxaTZQYjNzYjczIiwibm5jIjoiTnZURDhENWZjNXFpalIyMWJ1V2Z1ZE02dzlBM2drSy1ac3RtUW03b21pdyIsInByZiI6W119.QwZkb9R17tNhXnY_roqFYgdiIgUnSC18FYWQb3PcH6BU1R5l4W_T4XdACyczPGfM-jAnF2r2loBXDntYVS6N5A").unwrap(),
        "did:plc:s7onbyja7c3yrsgvholkls5b",
    );
    assert!(parse_did_from_jwt("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9").is_err());
}
