use crate::{debug_println, sctx};
use base64::{engine::general_purpose, Engine as _};
use paste::paste;
use serde_json_any_key::*;
use sqlx::{mysql::MySqlRow, Row};

pub use bigdecimal;
pub use sqlx::types::BigDecimal;

pub mod docker_job;
pub use docker_job::DockerJob;

pub mod knowledge_object;
pub use knowledge_object::KnowledgeObject;

pub mod crg;
pub use crg::ComputeResourceGroup;

const BIND_LIMIT: usize = 65535;

pub trait ORMUpdatableFieldValue {
    fn get_changeset_value(&self) -> String;
}

impl ORMUpdatableFieldValue for String {
    fn get_changeset_value(&self) -> String {
        general_purpose::STANDARD.encode(self.clone())
    }
}

impl ORMUpdatableFieldValue for i32 {
    fn get_changeset_value(&self) -> String {
        self.to_string()
    }
}

impl ORMUpdatableFieldValue for i64 {
    fn get_changeset_value(&self) -> String {
        self.to_string()
    }
}

impl ORMUpdatableFieldValue for f32 {
    fn get_changeset_value(&self) -> String {
        self.to_string()
    }
}

impl ORMUpdatableFieldValue for f64 {
    fn get_changeset_value(&self) -> String {
        self.to_string()
    }
}

impl ORMUpdatableFieldValue for bool {
    fn get_changeset_value(&self) -> String {
        if *self == true {
            "1".to_string()
        } else {
            "0".to_string()
        }
    }
}

impl ORMUpdatableFieldValue for BigDecimal {
    fn get_changeset_value(&self) -> String {
        self.to_string()
    }
}

impl ORMUpdatableFieldValue for serde_json::Value {
    fn get_changeset_value(&self) -> String {
        self.to_string()
    }
}

impl<T> ORMUpdatableFieldValue for Option<T>
where
    T: ORMUpdatableFieldValue,
{
    fn get_changeset_value(&self) -> String {
        match self {
            Some(value) => value.get_changeset_value(),
            None => "NULL".to_string(),
        }
    }
}

pub trait ORMObject {
    fn id(&self) -> i32;
    fn metadata_id(&self) -> i32;
    fn name(&self) -> String;
    fn set_name(&mut self, name: String);
    fn description(&self) -> Option<String>;
    fn set_description(&mut self, description: String);
    fn get_changeset(&mut self) -> &mut Vec<(String, String)>;
    fn table_name() -> String;
    fn new_from_row(row: MySqlRow) -> Self;
}

macro_rules! orm_object_getter {
    ($name:ident, $field:ident, $type:ty) => {
        pub fn $field(&self) -> $type {
            self.$field.clone()
        }
    };
}
pub(crate) use orm_object_getter;

macro_rules! orm_object_setter {
    ($name:ident, $field:ident, $type:ty) => {
        paste! {
            pub fn [< set_ $field >] (&mut self, $field: $type) {
                self.$field = $field.clone();
                let value = $field.get_changeset_value();
                self._changeset.push((stringify!($field).to_string(), value));
            }
        }
    };
}
pub(crate) use orm_object_setter;

macro_rules! impl_orm_object {
    ($name:ident, $table_name:expr, $($field:ident: $type:ty),*) => {
        #[derive(Debug)]
        pub struct $name {
            _changeset: Vec<(String, String)>,
            id: i32,
            metadata_id: i32,
            name: String,
            description: Option<String>,
            $($field: $type),*
        }

        impl ORMObject for $name {
            fn get_changeset(&mut self) -> &mut Vec<(String, String)> {
                &mut self._changeset
            }

            fn table_name() -> String {
                $table_name.to_string()
            }

            fn id(&self) -> i32 {
                self.id
            }

            fn metadata_id(&self) -> i32 {
                self.metadata_id
            }
            fn name(&self) -> String {
                self.name.clone()
            }
            fn set_name(&mut self, name: String) {
                self.name = name.clone();
                self._changeset.push(("name".to_string(), general_purpose::STANDARD.encode(name)));
            }
            fn description(&self) -> Option<String> {
                self.description.clone()
            }
            fn set_description(&mut self, description: String) {
                self.description = Some(description.clone());
                self._changeset.push(("description".to_string(), general_purpose::STANDARD.encode(description)));
            }
            fn new_from_row(row: MySqlRow) -> Self {
                $name {
                    _changeset: Vec::new(),
                    id: row.get("id"),
                    metadata_id: row.get("metadata_id"),
                    name: row.get("name"),
                    description: row.get("description"),
                    $($field: row.try_get_unchecked(stringify!($field)).expect("uh oh")),*
                }
            }
        }

        impl $name {
            $(
                orm_object_getter!($name, $field, $type);
                orm_object_setter!($name, $field, $type);
            )*
        }
    };
}
pub(crate) use impl_orm_object;

pub async fn find_by_id<T: ORMObject>(
    sctx: &mut sctx::SecurityContext,
    id: i32,
) -> Result<T, Box<dyn std::error::Error>> {
    let table_name = T::table_name();
    debug_println!("Table name: {}", table_name);

    let query = format!("SELECT * FROM v_{} WHERE id = ?", table_name);
    debug_println!("Query: {}", query);

    let result = sqlx::query(&query)
        .bind(id)
        .fetch_optional(&sctx.pool)
        .await;

    debug_println!("Result: {:?}", result);

    match result {
        Ok(row) => match row {
            Some(row) => Ok(T::new_from_row(row)),
            None => {
                debug_println!("No row found");
                Err("No row found".into())
            }
        },
        Err(e) => {
            debug_println!("Error: {}", e);
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
    let obid = ob.id();
    let changeset = ob.get_changeset();
    let changeset_json = format!("[{}]", changeset.to_json_map().unwrap());
    debug_println!("changeset {} {}", query, changeset_json);
    let result = sqlx::query(&query)
        .bind(obid)
        .bind(changeset_json)
        .execute(&sc.pool)
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
        let result = sqlx::query(query).bind(id).fetch_optional(&sctx.pool).await;

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
            .execute(&sctx.pool)
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RealtimeMessage {
    #[serde(rename = "v_id")]
    pub id: i32,
    #[serde(rename = "v_via")]
    pub via: String,
    #[serde(rename = "v_by")]
    pub sent_by: String,
    #[serde(rename = "v_for")]
    pub sent_for: String,
    #[serde(rename = "v_payload")]
    pub payload: String,
    #[serde(rename = "v_ticket")]
    pub ticket: String,
    #[serde(rename = "v_created_at")]
    pub created_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct RealtimeMessageTicket {
    pub ticket: String,
    pub ko_id: i32,
    pub creator_user_id: i32,
    pub created_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
}

impl RealtimeMessageTicket {
    pub async fn new_from_ticket(
        sctx: &mut sctx::SecurityContext,
        ticket: String,
    ) -> Result<RealtimeMessageTicket, Box<dyn std::error::Error>> {
        if sctx.is_admin != true {
            return Err("Admin context required".into());
        }

        let query = "SELECT * FROM realtime_message_ticket WHERE ticket = ?";
        let result = sqlx::query(query)
            .bind(ticket)
            .fetch_optional(&sctx.pool)
            .await;

        match result {
            Ok(row) => match row {
                Some(row) => Ok(RealtimeMessageTicket {
                    ticket: row.get("ticket"),
                    ko_id: row.get("ko_id"),
                    creator_user_id: row.get("creator_user_id"),
                    created_at: row.get("created_at"),
                }),
                None => Err("No row found".into()),
            },
            Err(e) => Err(e.into()),
        }
    }
}

impl RealtimeMessage {
    pub async fn new_from_id(
        sctx: &mut sctx::SecurityContext,
        id: i32,
    ) -> Result<RealtimeMessage, Box<dyn std::error::Error>> {
        let query = "SELECT * FROM v_realtime_message WHERE id = ?";
        let result = sqlx::query(query).bind(id).fetch_optional(&sctx.pool).await;

        match result {
            Ok(row) => match row {
                Some(row) => Ok(RealtimeMessage {
                    id: row.get("id"),
                    via: row.get("via"),
                    sent_by: row.get("sent_by"),
                    sent_for: row.get("sent_for"),
                    payload: row.get("payload"),
                    ticket: row.get("ticket"),
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
        let query = "CALL sp_send_message_to_processor (?)";
        let result = sqlx::query(query).bind(payload).execute(&sctx.pool).await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn send_to_self(
        sctx: &mut sctx::SecurityContext,
        payload: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let query = "CALL sp_user_send_realtime_message (?)";
        let result = sqlx::query(query).bind(payload).execute(&sctx.pool).await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn send_to_ko(
        sctx: &mut sctx::SecurityContext,
        ko_id: i32,
        ticket: String,
        payload: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let query = "CALL sp_ko_send_realtime_message (?, ?, ?)";
        let result = sqlx::query(query)
            .bind(ticket)
            .bind(ko_id)
            .bind(payload)
            .execute(&sctx.pool)
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn consume_queue(
        sctx: &mut sctx::SecurityContext,
        count: i32,
    ) -> Result<Vec<RealtimeMessage>, Box<dyn std::error::Error>> {
        if sctx.is_admin != true {
            return Err("Admin context required".into());
        }

        let query = "CALL sp_consume_realtime_message_queue (?)";
        let result = sqlx::query(query).bind(count).fetch_all(&sctx.pool).await;
        let mut messages = Vec::new();
        for row in result? {
            // sqlx does not support getting by column name from rows returned by stored procedures
            messages.push(RealtimeMessage {
                id: row.get(0),
                via: row.get(1),
                sent_by: row.get(2),
                sent_for: row.get(3),
                ticket: match row.try_get(4) {
                    Ok(ticket) => ticket,
                    Err(_) => "".to_string(),
                },
                payload: row.get(5),
                created_at: row.get(6),
            });
        }

        Ok(messages)
    }
}

pub async fn transact_credits(
    sctx: &mut sctx::SecurityContext,
    amount: BigDecimal,
    statement: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let query = "CALL sp_transact_credits (NULL, ?, ?)";
    let result = sqlx::query(query)
        .bind(amount)
        .bind(statement)
        .execute(&sctx.pool)
        .await;

    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

pub async fn wait_for_cdc_event(
    sctx: &mut sctx::SecurityContext,
    event: String,
    seconds: i32,
) -> bool {
    // SELECT /* WAITING_FOR_EVENT ({}) */ SLEEP({})".format(event,s)
    // if query is killed (2013: Lost connection to MySQL server during query), return true
    // otherwise, false

    let query = format!(
        "SELECT /* WAITING_FOR_EVENT ({}) */ SLEEP({})",
        event, seconds
    );
    let result = sqlx::query(&query).execute(&sctx.pool).await;

    match result {
        Ok(_) => false,
        Err(e) => e
            .to_string()
            .contains("Lost connection to MySQL server during query"),
    }
}
