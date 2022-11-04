use serde;

#[allow(non_snake_case)]
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct AccountRequest {
    pub email: String,
    pub username: String,
    pub password: String,
    pub inviteCode: Option<String>,
    pub recoveryKey: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct SessionRequest {
    pub username: String,
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
