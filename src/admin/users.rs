use sqlx::{mysql, types::chrono};

use crate::{debug_println, sctx};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub consented: bool,
    pub organization_id: i32,
    pub jwt_secret: String,
    pub salt: String,
}

pub async fn find_user_by_email(
    sctx: &mut sctx::SecurityContext,
    email: &str,
) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT
	u.id,
	u.username,
	d.email,
	d.first_name,
	d.last_name,
	d.consented,
	u.organization_id,
	w.jwt_secret,
    w.salt
FROM miranda.users_details d
LEFT JOIN miranda.users u ON u.id = d.user_id
INNER JOIN miranda_web.web_users w on w.username = u.username
WHERE d.email = ?",
    )
    .bind(email)
    .fetch_one(&sctx.pool)
    .await
}

pub async fn find_user_by_username(
    sctx: &mut sctx::SecurityContext,
    username: &str,
) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT
	u.id,
	u.username,
	d.email,
	d.first_name,
	d.last_name,
	d.consented,
	u.organization_id,
	w.jwt_secret,
    w.salt
FROM miranda.users_details d
LEFT JOIN miranda.users u ON u.id = d.user_id
INNER JOIN miranda_web.web_users w on w.username = u.username
WHERE u.username = ?",
    )
    .bind(username)
    .fetch_one(&sctx.pool)
    .await
}
