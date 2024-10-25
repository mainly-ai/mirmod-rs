use super::*;
use base64::{engine::general_purpose, Engine as _};

#[derive(Clone, Debug, PartialEq, PartialOrd, sqlx::Type, serde::Deserialize, serde::Serialize)]
#[sqlx(type_name = "workflow_state", rename_all = "UPPERCASE")]
pub enum WorkflowState {
    Uninitialized,
    Starting,
    Ready,
    ResumeReady,
    Running,
    Error,
    Exited,
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
    DockerJob,
    "docker_job",
    workflow_state: WorkflowState,
    gpu_capacity: f32,
    cpu_seconds: f32,
    current_cpu: f32,
    cpu_capacity: f32,
    ram_gb_seconds: f32,
    current_ram_gb: f32,
    ram_gb_capacity: f32,
    net_rx_gb: f32,
    current_net_rx_gb: f32,
    net_tx_gb: f32,
    current_net_tx_gb: f32,
    total_cost: f32,
    crg_id: Option<i32>
);
