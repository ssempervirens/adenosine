/// app.bsky types (manually entered)
use serde_json::Value;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct Subject {
    pub uri: String,
    // TODO: CID is required
    pub cid: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct StrongRef {
    pub uri: String,
    pub cid: String,
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
pub struct ProfileView {
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
    pub viewer: serde_json::Value,
}

/// for Timeline or AuthorFeed
#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct GenericFeed {
    pub feed: Vec<FeedPostView>,
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
pub struct UserView {
    pub did: String,
    pub handle: String,
    pub declaration: DeclRef,
    pub displayName: Option<String>,
    pub avatar: Option<String>,
    pub viewer: Option<Value>,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct PostView {
    pub uri: String,
    pub cid: String,
    pub author: UserView,
    pub record: Post,
    pub embed: Option<PostEmbedView>,
    pub replyCount: u64,
    pub repostCount: u64,
    pub upvoteCount: u64,
    pub downvoteCount: u64,
    pub indexedAt: String,
    pub viewer: Option<Value>,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct ThreadPostView {
    // TODO: doing this as the intersetion of #threadViewPost and #notFoundPost. actually it is
    // supposed to be a union type
    // #notFoundPost fields (uri and notFound actually required)
    pub uri: Option<String>,
    pub notFound: Option<bool>,
    // #threadViewPost fields (post actually required)
    pub post: Option<PostView>,
    pub parent: Option<Box<ThreadPostView>>,
    pub replies: Option<Vec<ThreadPostView>>,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct FeedPostView {
    pub post: PostView,
    pub reply: Option<PostReply>,
    // TODO: this could extend to other "reasons" in the future
    pub reason: Option<RepostReason>,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct RepostReason {
    pub by: UserView,
    pub indexedAt: String,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct Post {
    pub text: String,
    pub reply: Option<PostReply>,
    pub entities: Option<Vec<PostEntity>>,
    pub embed: Option<PostEmbed>,
    pub createdAt: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct PostReply {
    // TODO: these should be StrongRef
    pub parent: Subject,
    pub root: Subject,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct PostEntity {
    pub index: TextSlice,
    pub r#type: String,
    pub value: String,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct TextSlice {
    pub start: u64,
    pub end: u64,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct PostEmbed {
    pub external: Option<EmbedExternal>,
    pub images: Option<Vec<EmbedImage>>,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct PostEmbedView {
    pub external: Option<EmbedExternalView>,
    pub images: Option<Vec<EmbedImageView>>,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct EmbedExternal {
    pub uri: String,
    pub title: String,
    pub description: String,
    pub thumb: Option<Blob>,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct EmbedExternalView {
    pub uri: String,
    pub title: String,
    pub description: String,
    pub thumb: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct EmbedImage {
    pub image: Blob,
    pub alt: String,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct Blob {
    pub cid: String,
    pub mimeType: String,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct EmbedImageView {
    pub thumb: String,
    pub fullsize: String,
    pub alt: String,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct PostThread {
    pub thread: ThreadPostView,
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
