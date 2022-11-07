
use adenosine_cli::identifiers::{Did, Nsid, Tid};
use serde_json;
use askama::Template;
use crate::repo::RepoCommit;
use crate::models::*;

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
#[template(path = "profile.html")]
pub struct ProfileView {
    pub domain: String,
    pub did: Did,
    pub profile: serde_json::Value,
    pub feed: Vec<serde_json::Value>,
}

#[derive(Template)]
#[template(path = "post.html")]
pub struct PostView {
    pub domain: String,
    pub did: Did,
    pub collection: Nsid,
    pub tid: Tid,
    pub post_text: String,
    pub post_created_at: String,
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
