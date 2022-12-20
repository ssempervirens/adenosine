use serde_json::Value;

// =========== com.atproto types (manually entered)

#[allow(non_snake_case)]
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct AccountRequest {
    pub email: String,
    pub handle: String,
    pub password: String,
    pub inviteCode: Option<String>,
    pub recoveryKey: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct SessionRequest {
    pub handle: String,
    pub password: String,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct AtpSession {
    pub did: String,
    pub name: String,
    pub accessJwt: String,
    pub refreshJwt: String,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct RepoDescribe {
    pub name: String,
    pub did: String,
    pub didDoc: serde_json::Value,
    pub collections: Vec<String>,
    pub nameIsCorrect: bool,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct RepoCreateRecord {
    pub did: String,
    pub collection: String,
    pub record: serde_json::Value,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct RepoPutRecord {
    pub did: String,
    pub collection: String,
    pub rkey: String,
    pub record: serde_json::Value,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct RepoDeleteRecord {
    pub did: String,
    pub collection: String,
    pub rkey: String,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct RepoBatchWriteBody {
    pub did: String,
    pub writes: Vec<RepoBatchWrite>,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct RepoBatchWrite {
    #[serde(rename = "type")]
    pub op_type: String,
    pub collection: String,
    pub rkey: Option<String>,
    pub value: serde_json::Value,
}

// =========== app.bsky types (manually entered)

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct Subject {
    pub uri: String,
    // TODO: CID is required
    pub cid: Option<String>,
}

/// Generic over Re-post and Like
#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct RefRecord {
    pub subject: Subject,
    pub createdAt: String,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct FollowSubject {
    pub did: String,
    // pub declarationCid: String,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct FollowRecord {
    pub subject: FollowSubject,
    pub createdAt: String,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct ProfileRecord {
    pub displayName: String,
    pub description: Option<String>,
}

// app.bsky.system.actorUser or app.bsky.system.actorScene
#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub actorType: String,
}

// actorType: app.bsky.system.actorUser
// cid: bafyreid27zk7lbis4zw5fz4podbvbs4fc5ivwji3dmrwa6zggnj4bnd57u
#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct DeclRef {
    pub actorType: String,
    pub cid: String,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct Profile {
    pub did: String,
    pub declaration: DeclRef,
    pub handle: String,
    // for simple accounts, 'creator' is just the did
    pub creator: String,
    pub displayName: Option<String>,
    pub description: Option<String>,
    pub followersCount: u64,
    pub followsCount: u64,
    pub membersCount: u64,
    pub postsCount: u64,
    pub myState: serde_json::Value,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct GenericFeed {
    pub feed: Vec<FeedItem>,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct User {
    pub did: String,
    pub handle: String,
    pub displayName: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct FeedItem {
    pub uri: String,
    // TODO: cid is required
    pub cid: Option<String>,
    pub author: User,
    pub repostedBy: Option<User>,
    pub record: Value,
    //pub embed?: RecordEmbed | ExternalEmbed | UnknownEmbed,
    pub embed: Option<Value>,
    pub replyCount: u64,
    pub repostCount: u64,
    pub upvoteCount: u64,
    pub downvoteCount: u64,
    pub indexedAt: String,
    pub myState: Option<Value>,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct Post {
    pub text: String,
    pub reply: Option<PostReply>,
    pub createdAt: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct PostReply {
    pub parent: Subject,
    pub root: Subject,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct PostThread {
    pub thread: ThreadItem,
}

// TODO: 'parent' and 'replies' should allow "NotFoundPost" for references that point to an unknown
// URI
#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct ThreadItem {
    pub uri: String,
    // TODO: CID is required
    pub cid: Option<String>,
    pub author: User,
    pub record: Value,
    //pub embed?: RecordEmbed | ExternalEmbed | UnknownEmbed,
    pub embed: Option<Value>,
    pub parent: Option<Box<ThreadItem>>,
    pub replyCount: u64,
    pub replies: Option<Vec<ThreadItem>>,
    pub repostCount: u64,
    pub upvoteCount: u64,
    pub downvoteCount: u64,
    pub indexedAt: String,
    pub myState: Option<Value>,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct FollowTarget {
    // TODO: nested follow list?
    pub subject: Subject,
    pub did: String,
    pub handle: String,
    pub displayName: Option<String>,
    pub createdAt: Option<String>,
    pub indexedAt: String,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct Follow {
    // TODO: nested follow list?
    pub subject: Subject,
    pub follows: FollowTarget,
}
