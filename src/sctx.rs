use crate::config;
use crate::debug_println;
use sqlx::{Connection, Row};
use std::env;

pub struct SecurityContext {
    pub user_id: i32,
    pub auth_string: [String; 2],
    pub conn: sqlx::MySqlConnection,
}

impl SecurityContext {
    pub async fn new(
        username: &str,
        password: &str,
        host: &str,
        port: &i32,
        database: &str,
    ) -> Result<SecurityContext, Box<dyn std::error::Error>> {
        let conn = sqlx::MySqlConnection::connect(&format!(
            "mysql://{}:{}@{}:{}/{}",
            username, password, host, port, database
        ))
        .await?;
        let auth_string = [username.to_string(), password.to_string()];
        Ok(SecurityContext {
            user_id: -1,
            auth_string,
            conn,
        })
    }

    pub async fn new_from_config(
        config: config::MirandaConfig,
    ) -> Result<SecurityContext, Box<dyn std::error::Error>> {
        let port = config.port.parse::<i32>()?;
        let sc = SecurityContext::new(
            &config.user,
            &config.password,
            &config.host,
            &port,
            &config.database,
        )
        .await?;
        Ok(sc)
    }

    pub async fn extend_proxy_account_claim(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let app_name = env::var("MIRANDA_APPLICATION").unwrap_or_else(|_| "mirmod-rs".to_string());
        debug_println!("[sctx] Extending proxy account claim for {}", app_name);
        let row = sqlx::query("CALL sp_extend_proxy_account_claim(?)")
            .bind(app_name)
            .execute(&mut self.conn)
            .await;

        match row {
            Ok(_) => Ok(()),
            Err(e) => {
                debug_println!("[sctx] Error extending proxy account claim: {}", e);
                Err(e.into())
            }
        }
    }

    pub async fn renew_id(&mut self) -> Result<i32, Box<dyn std::error::Error>> {
        debug_println!("[sctx] Renewing id");
        let row = sqlx::query("SELECT * FROM v_user")
            .fetch_optional(&mut self.conn)
            .await;
        match row {
            Ok(Some(row)) => {
                self.user_id = row.get::<i32, &str>("id");
                println!("[sctx] id={}", self.user_id);
                if self.auth_string[0].starts_with("pxy.") {
                    let claim = self.extend_proxy_account_claim().await;
                    match claim {
                        Ok(_) => Ok(self.user_id),
                        Err(e) => Err(e),
                    }
                } else {
                    Ok(self.user_id)
                }
            }
            Ok(None) => {
                self.user_id = -1;
                Err("No user found".into())
            }
            Err(e) => Err(e.into()),
        }
    }
}
