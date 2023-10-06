use crate::sctx;
use base64::{engine::general_purpose, Engine as _};
use serde_json_any_key::*;
use sqlx::{mysql::MySqlRow, Row};

const BIND_LIMIT: usize = 65535;

pub trait ORMObject {
    fn get_id(&self) -> i32;
    fn get_changeset(&mut self) -> &mut Vec<(String, String)>;
    fn table_name() -> String;
    fn new_from_row(row: MySqlRow) -> Self;
}

#[derive(Debug)]
pub struct BaseObject {
    _changeset: Vec<(String, String)>,
    metadata_id: i32,
    name: String,
    description: Option<String>,
}

impl BaseObject {
    fn set_name(&mut self, name: String) {
        self.name = name.clone();
        self._changeset
            .push(("name".to_string(), general_purpose::STANDARD.encode(name)));
    }

    fn set_description(&mut self, description: String) {
        self.description = Some(description.clone());
        self._changeset.push((
            "description".to_string(),
            general_purpose::STANDARD.encode(description),
        ));
    }
}

#[derive(Debug, Clone)]
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
}

impl DockerJob {
    pub fn set_workflow_state(&mut self, workflow_state: DockerJobWorkflowState) {
        self.workflow_state = workflow_state.clone();
        self.base._changeset.push((
            "workflow_state".to_string(),
            general_purpose::STANDARD.encode(workflow_state.as_str()),
        ));
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
        }
    }
}

pub async fn find_by_id<T: ORMObject>(
    sctx: &mut sctx::SecurityContext,
    id: i32,
) -> Result<T, Box<dyn std::error::Error>> {
    let table_name = T::table_name();
    println!("Table name: {}", table_name);

    let query = format!("SELECT * FROM v_{} WHERE id = ?", table_name);
    println!("Query: {}", query);

    let result = sqlx::query(&query)
        .bind(id)
        .fetch_optional(&mut sctx.conn)
        .await;

    match result {
        Ok(row) => match row {
            Some(row) => Ok(T::new_from_row(row)),
            None => {
                println!("No row found");
                Err("No row found".into())
            }
        },
        Err(e) => {
            println!("Error: {}", e);
            Err(e.into())
        }
    }
}

pub async fn update<T: ORMObject>(
    sc: &mut sctx::SecurityContext,
    ob: &mut T,
) -> Result<(), Box<dyn std::error::Error>> {
    let table_name = T::table_name();
    let query = format!("CALL sp_update_{} (?, ?)", table_name);
    let obid = ob.get_id();
    let changeset = ob.get_changeset();
    let changeset_json = format!("[{}]", changeset.to_json_map().unwrap());
    println!("Changeset: {}", changeset_json);
    let result = sqlx::query(&query)
        .bind(obid)
        .bind(changeset_json)
        .execute(&mut sc.conn)
        .await;

    match result {
        Ok(_) => {
            changeset.clear();
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

pub enum MirandaClasses {
    DockerJob = 1,
}

pub fn get_class_id(id: i32) -> MirandaClasses {
    match id {
        1 => MirandaClasses::DockerJob,
        _ => panic!("Invalid class id"),
    }
}

pub struct MirandaLog {
    id: i32,
    created_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
    message: String,
    tag: i32,
    class_id: MirandaClasses,
    instance_id: i32,
}

impl MirandaLog {
    pub async fn new_from_id(
        sctx: &mut sctx::SecurityContext,
        id: i32,
    ) -> Result<MirandaLog, Box<dyn std::error::Error>> {
        let query = "SELECT * FROM v_miranda_log WHERE id = ?";
        let result = sqlx::query(query)
            .bind(id)
            .fetch_optional(&mut sctx.conn)
            .await;

        match result {
            Ok(row) => match row {
                Some(row) => Ok(MirandaLog {
                    id: row.get("id"),
                    created_at: row.get("created_at"),
                    message: row.get("message"),
                    tag: row.get("tag"),
                    class_id: get_class_id(row.get("class_id")),
                    instance_id: row.get("instance_id"),
                }),
                None => Err("No row found".into()),
            },
            Err(e) => Err(e.into()),
        }
    }

    pub async fn create(
        sctx: &mut sctx::SecurityContext,
        message: String,
        tag: i64,
        class_id: MirandaClasses,
        instance_id: i64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let query = "CALL sp_log (?, ?, ?, ?)";
        let result = sqlx::query(query)
            .bind(class_id as i64)
            .bind(instance_id)
            .bind(tag)
            .bind(message)
            .execute(&mut sctx.conn)
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

pub struct RealtimeMessage {
    id: i32,
    sent_by: String,
    sent_for: String,
    payload: String,
    created_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
}

impl RealtimeMessage {
    pub async fn new_from_id(
        sctx: &mut sctx::SecurityContext,
        id: i32,
    ) -> Result<RealtimeMessage, Box<dyn std::error::Error>> {
        let query = "SELECT * FROM v_realtime_message WHERE id = ?";
        let result = sqlx::query(query)
            .bind(id)
            .fetch_optional(&mut sctx.conn)
            .await;

        match result {
            Ok(row) => match row {
                Some(row) => Ok(RealtimeMessage {
                    id: row.get("id"),
                    sent_by: row.get("sent_by"),
                    sent_for: row.get("sent_for"),
                    payload: row.get("payload"),
                    created_at: row.get("created_at"),
                }),
                None => Err("No row found".into()),
            },
            Err(e) => Err(e.into()),
        }
    }

    pub async fn send_to_processor(
        sctx: &mut sctx::SecurityContext,
        payload: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if payload.len() > 15360 {
            return Err("Payload too large".into());
        }
        let query = "CALL sp_send_message_to_processor (?)";
        let result = sqlx::query(query)
            .bind(payload)
            .execute(&mut sctx.conn)
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn send_to_self(
        sctx: &mut sctx::SecurityContext,
        payload: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if payload.len() > 15360 {
            return Err("Payload too large".into());
        }
        let query = "CALL sp_user_send_realtime_message (?)";
        let result = sqlx::query(query)
            .bind(payload)
            .execute(&mut sctx.conn)
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}
