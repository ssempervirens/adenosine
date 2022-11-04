use serde;

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct AtpSession {
    pub did: String,
    pub name: String,
    pub accessJwt: String,
    pub refreshJwt: String,
}
