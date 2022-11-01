use serde;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct AtpSession {
    pub jwt: String,
    pub name: String,
    pub did: String,
}
