use crate::config;
use crate::debug_println;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{Connection, Row};
use std::env;

#[derive(Clone, Debug)]
pub struct SecurityContext {
    pub user_id: i32,
    pub auth_string: [String; 2],
    pub pool: sqlx::Pool<sqlx::MySql>,
    pub is_admin: bool,
}

impl SecurityContext {
    pub async fn new(
        username: &str,
        password: &str,
        host: &str,
        port: &i32,
        database: &str,
    ) -> Result<SecurityContext, Box<dyn std::error::Error>> {
        let connstr = format!(
            "mysql://{}:{}@{}:{}/{}",
            username, password, host, port, database
        );
        let pool = match MySqlPoolOptions::new()
            .max_connections(1)
            .test_before_acquire(false)
            .before_acquire(|conn, meta| {
                Box::pin(async move {
                    if meta.idle_for.as_secs() > 15 {
                        debug_println!("[sctx] Idle for more than 15 seconds, checking connection");
                        let res = conn.ping().await;
                        match res {
                            Ok(_) => {
                                debug_println!("[sctx] Connection is alive");
                            }
                            Err(e) => {
                                debug_println!("[sctx] Connection is dead, Reconnecting... {}", e);
                                return Ok(false);
                            }
                        }
                    }

                    Ok(true)
                })
            })
            .connect(&connstr)
            .await
        {
            Ok(pool) => pool,
            Err(e) => {
                debug_println!("[sctx] Error connecting to database: {}", e);
                return Err(e.into());
            }
        };
        let auth_string = [username.to_string(), password.to_string()];
        Ok(SecurityContext {
            user_id: -1,
            auth_string,
            pool,
            is_admin: false,
        })
    }

    pub fn set_admin(&mut self, is_admin: bool) {
        self.is_admin = is_admin;
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
            .execute(&self.pool)
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
        if self.is_admin {
            return Ok(-1);
        }
        let row = sqlx::query("SELECT * FROM v_user")
            .fetch_optional(&self.pool)
            .await;
        match row {
            Ok(Some(row)) => {
                self.user_id = row.get::<i32, &str>("id");
                debug_println!("[sctx] id={}", self.user_id);
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
