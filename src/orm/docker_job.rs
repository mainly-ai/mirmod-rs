use base64::{engine::general_purpose, Engine as _};

use super::*;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DockerJobWorkflowState {
    Uninitialized,
    Starting,
    Ready,
    Running,
    Error,
    Exited,
    ResumeReady,
}

impl DockerJobWorkflowState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Uninitialized => "UNINITIALIZED",
            Self::Starting => "STARTING",
            Self::Ready => "READY",
            Self::Running => "RUNNING",
            Self::Error => "ERROR",
            Self::Exited => "EXITED",
            Self::ResumeReady => "RESUME_READY",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "UNINITIALIZED" => Self::Uninitialized,
            "STARTING" => Self::Starting,
            "READY" => Self::Ready,
            "RUNNING" => Self::Running,
            "ERROR" => Self::Error,
            "EXITED" => Self::Exited,
            "RESUME_READY" => Self::ResumeReady,
            _ => panic!("Invalid workflow state"),
        }
    }
}

#[derive(Debug)]
pub struct DockerJob {
    pub base: BaseObject,
    pub id: i32,
    pub workflow_state: DockerJobWorkflowState,
    pub cpu_seconds: f32,
    pub ram_gb_seconds: f32,
    pub net_tx_gb: f32,
    pub net_rx_gb: f32,
}

impl DockerJob {
    pub fn set_workflow_state(&mut self, workflow_state: DockerJobWorkflowState) {
        self.workflow_state = workflow_state.clone();
        self.base._changeset.push((
            "workflow_state".to_string(),
            general_purpose::STANDARD.encode(workflow_state.as_str()),
        ));
    }

    pub fn set_cpu_seconds(&mut self, cpu_seconds: f32) {
        self.cpu_seconds = cpu_seconds;
        self.base
            ._changeset
            .push(("cpu_seconds".to_string(), cpu_seconds.to_string()));
    }

    pub fn set_ram_gb_seconds(&mut self, ram_gb_seconds: f32) {
        self.ram_gb_seconds = ram_gb_seconds;
        self.base
            ._changeset
            .push(("ram_gb_seconds".to_string(), ram_gb_seconds.to_string()));
    }

    pub fn set_net_tx_gb(&mut self, net_tx_gb: f32) {
        self.net_tx_gb = net_tx_gb;
        self.base
            ._changeset
            .push(("net_tx_gb".to_string(), net_tx_gb.to_string()));
    }

    pub fn set_net_rx_gb(&mut self, net_rx_gb: f32) {
        self.net_rx_gb = net_rx_gb;
        self.base
            ._changeset
            .push(("net_rx_gb".to_string(), net_rx_gb.to_string()));
    }
}

impl ORMObject for DockerJob {
    fn get_id(&self) -> i32 {
        self.id
    }
    fn get_changeset(&mut self) -> &mut Vec<(String, String)> {
        &mut self.base._changeset
    }
    fn table_name() -> String {
        String::from("docker_job")
    }
    fn new_from_row(row: MySqlRow) -> Self {
        DockerJob {
            base: BaseObject {
                _changeset: Vec::new(),
                metadata_id: row.get("metadata_id"),
                name: row.get("name"),
                description: row.get("description"),
            },
            id: row.get("id"),
            workflow_state: DockerJobWorkflowState::from_str(row.get("workflow_state")),
            cpu_seconds: row.get("cpu_seconds"),
            ram_gb_seconds: row.get("ram_gb_seconds"),
            net_tx_gb: row.get("net_tx_gb"),
            net_rx_gb: row.get("net_rx_gb"),
        }
    }
}
