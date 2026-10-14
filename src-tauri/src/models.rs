use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatus {
    pub configured: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetupWorkspaceInput {
    pub organization_name: String,
    pub counselor_name: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInfo {
    pub organization_id: String,
    pub organization_name: String,
    pub counselor_id: String,
    pub counselor_name: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Client {
    pub id: String,
    pub organization_id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub household_size: u32,
    pub status: String,
    pub goals: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClientInput {
    pub id: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub household_size: u32,
    pub status: String,
    pub goals: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientNote {
    pub id: String,
    pub client_id: String,
    pub counselor_id: String,
    pub counselor_name: String,
    pub body: String,
    pub created_at: String,
}
