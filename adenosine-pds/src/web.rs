use crate::models::*;
use adenosine::identifiers::{Did, Nsid, Tid};
use adenosine::repo::RepoCommit;
use askama::Template;

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorView {
    pub domain: String,
    pub status_code: u16,
    pub error_message: String,
}

#[derive(Template)]
#[template(path = "home.html")]
pub struct GenericHomeView {
    pub domain: String,
}

#[derive(Template)]
#[template(path = "about.html")]
pub struct AboutView {
    pub domain: String,
}

#[derive(Template)]
#[template(path = "account.html")]
pub struct AccountView {
    pub domain: String,
    pub did: Did,
    pub profile: Profile,
    pub feed: Vec<FeedItem>,
}

#[derive(Template)]
#[template(path = "thread.html")]
pub struct ThreadView {
    pub domain: String,
    pub did: Did,
    pub collection: Nsid,
    pub tid: Tid,
    pub post: ThreadItem,
}

#[derive(Template)]
#[template(path = "at_repo.html")]
pub struct RepoView {
    pub domain: String,
    pub did: Did,
    pub commit: RepoCommit,
    pub describe: RepoDescribe,
}

#[derive(Template)]
#[template(path = "at_collection.html")]
pub struct CollectionView {
    pub domain: String,
    pub did: Did,
    pub collection: Nsid,
    pub records: Vec<serde_json::Value>,
}

#[derive(Template)]
#[template(path = "at_record.html")]
pub struct RecordView {
    pub domain: String,
    pub did: Did,
    pub collection: Nsid,
    pub tid: Tid,
    pub record: serde_json::Value,
}

mod filters {
    use crate::AtUri;
    use std::str::FromStr;

    pub fn aturi_to_path(aturi: &str) -> ::askama::Result<String> {
        let aturi = AtUri::from_str(aturi).expect("aturi parse");
        if aturi.record.is_some() {
            Ok(format!(
                "/at/{}/{}/{}",
                aturi.repository,
                aturi.collection.unwrap(),
                aturi.record.unwrap()
            ))
        } else if aturi.collection.is_some() {
            Ok(format!(
                "/at/{}/{}",
                aturi.repository,
                aturi.collection.unwrap()
            ))
        } else {
            Ok(format!("/at/{}", aturi.repository))
        }
    }

    pub fn aturi_to_thread_path(aturi: &str) -> ::askama::Result<String> {
        let aturi = AtUri::from_str(aturi).expect("aturi parse");
        Ok(format!(
            "/u/{}/post/{}",
            aturi.repository,
            aturi.record.unwrap()
        ))
    }

    pub fn aturi_to_tid(aturi: &str) -> ::askama::Result<String> {
        let aturi = AtUri::from_str(aturi).expect("aturi parse");
        if aturi.record.is_some() {
            Ok(aturi.record.unwrap())
        } else {
            // TODO: raise an askama error here?
            Ok("<MISSING>".to_string())
        }
    }
}
