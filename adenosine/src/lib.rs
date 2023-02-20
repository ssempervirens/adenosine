pub mod app_bsky;
pub mod auth;
pub mod car;
pub mod com_atproto;
pub mod crypto;
pub mod identifiers;
pub mod ipld;
pub mod mst;
pub mod plc;
pub mod repo;
pub mod xrpc;

mod ucan_p256;
mod vendored;

/// Helper to generate the current timestamp as right now, UTC, formatted as an RFC 3339 string.
///
/// Currently, bluesky PDS expects millisecond precision, so we use that.
///
/// Returns something like "2022-11-22T09:21:15.640Z"
pub fn created_at_now() -> String {
    let now = time::OffsetDateTime::now_utc();
    // remove microsecond precision, but retain millisecond precision
    let ms = now.millisecond();
    let now = now.replace_microsecond(0).unwrap();
    let now = now.replace_millisecond(ms).unwrap();
    now.format(&time::format_description::well_known::Rfc3339)
        .unwrap()
}

#[test]
fn test_created_at_now() {
    // eg: 2022-11-22T09:20:44.123Z
    let ts = created_at_now();
    println!("{ts}");
    assert_eq!(&ts[4..5], "-");
    assert_eq!(&ts[7..8], "-");
    assert_eq!(&ts[10..11], "T");
    assert_eq!(&ts[23..24], "Z");
}
