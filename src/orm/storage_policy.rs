use super::*;

#[derive(Clone, Debug, PartialEq, PartialOrd, sqlx::Type, serde::Deserialize, serde::Serialize)]
#[sqlx(type_name = "storage_type", rename_all = "UPPERCASE")]
pub enum StorageType {
    Vault,
    Ssh,
}

impl ORMUpdatableFieldValue for StorageType {
    fn get_changeset_value(&self) -> String {
        general_purpose::STANDARD.encode(format!("{:?}", self).to_uppercase())
    }
}

impl StorageType {
    pub fn as_str(&self) -> String {
        format!("{:?}", self).to_uppercase()
    }
}


#[derive(Clone, Debug, PartialEq, PartialOrd, sqlx::Type, serde::Deserialize, serde::Serialize)]
#[sqlx(type_name = "workflow_state", rename_all = "UPPERCASE")]
pub enum WorkflowState {
    Unprovisioned,
    Unusable,
    Ready
}

impl ORMUpdatableFieldValue for WorkflowState {
    fn get_changeset_value(&self) -> String {
        general_purpose::STANDARD.encode(format!("{:?}", self).to_uppercase())
    }
}

impl WorkflowState {
    pub fn as_str(&self) -> String {
        format!("{:?}", self).to_uppercase()
    }
}


impl_orm_object!(
    StoragePolicy,
    "storage_policy",
    storage_type: StorageType,
    mount_point: String,
    details: String,
    workflow_state: WorkflowState
);
