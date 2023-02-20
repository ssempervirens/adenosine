/// com.atproto.repo types (manually entered)

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct Describe {
    pub name: String,
    pub did: String,
    pub didDoc: serde_json::Value,
    pub collections: Vec<String>,
    pub nameIsCorrect: bool,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct CreateRecord {
    pub did: String,
    pub collection: String,
    pub record: serde_json::Value,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct PutRecord {
    pub did: String,
    pub collection: String,
    pub rkey: String,
    pub record: serde_json::Value,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct DeleteRecord {
    pub did: String,
    pub collection: String,
    pub rkey: String,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct BatchWriteBody {
    pub did: String,
    pub writes: Vec<BatchWrite>,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct BatchWrite {
    #[serde(rename = "type")]
    pub op_type: String,
    pub collection: String,
    pub rkey: Option<String>,
    pub value: serde_json::Value,
}
