// com.atproto types (manually entered)

pub mod repo;

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
pub struct Session {
    pub did: String,
    pub name: String,
    pub accessJwt: String,
    pub refreshJwt: String,
}
